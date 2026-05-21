use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

pub mod io;
pub mod builder;
pub mod kernels;

use crate::grid::Grid;

use crate::boundary_conditions::BoundaryConditions;
use crate::actuator_line_interface::ActuatorLineInterface;
use crate::geometry::{
    Geometry,
    blending_function
};

use crate::pressure_solver::PressureSolverMultiGrid;

use rayon::prelude::*;

use std::time::Instant;

pub struct Simulation {
    pub velocity: Vec<SpatialVector>,
    pub velocity_org: Vec<SpatialVector>,
    pub velocity_star: Vec<SpatialVector>,
    pub body_force: Vec<SpatialVector>,
    pub boundary_conditions: BoundaryConditions,
    pub pressure_solver: PressureSolverMultiGrid,
    pub grid: Grid,
    pub viscosity: Float,
    pub density: Float,
    pub geometries: Vec<Geometry>,
    pub actuator_line: Option<ActuatorLineInterface>,
}

impl Simulation {
    pub fn initialize_after_build(&mut self) {
        println!("Initializing after build");
        self.actuator_line_initialization();
        
        self.correct_velocities_for_geometry();

        println!();
    }

    pub fn time_step_from_courant_number(&self, courant_number: Float) -> Float {
        let mut max_velocity = 0.0;
        for i in 0..self.velocity.len() {
            if self.velocity[i].length() > max_velocity {
                max_velocity = self.velocity[i].length();
            }
        }

        let cell_length = self.grid.cell_length;

        let mut min_cell_length = Float::INFINITY;
        for i in 0..3 {
            if cell_length[i] < min_cell_length {
                min_cell_length = cell_length[i];
            }
        }

        courant_number * min_cell_length / max_velocity
    }

    pub fn do_step(&mut self, time: Float, time_step: Float) {      
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid,
            &mut self.velocity
        );

        self.velocity_org.copy_from_slice(&self.velocity);

        for iteration in 0..2 {
            println!("Prediction {}", iteration+1);
            self.convect_and_diffuse(time_step);
            self.correct_velocities_for_geometry();
            self.project_pressure(time_step);
            self.update_velocity(time_step);
            self.correct_velocities_for_geometry();
        }
        
        self.run_actuator_line_model(time, time_step);
        
        if let Some(actuator_line) = &self.actuator_line {
            actuator_line.model.write_results("");
        }
        
        println!();
    }
    
    /// Computes the cell center velocity, by interpolating from the faces to the center
    pub fn cell_center_velocity_from_interior_indices(&self, interior_indices: [usize; 3]) -> SpatialVector {
        let [i, j, k] = self.grid.extended_indices_from_interior_indices(interior_indices);
        
        let i_0 = self.grid.flat_index_on_extended_grid([i, j, k]);
        
        let i_n = [
            self.grid.flat_index_on_extended_grid([i-1, j, k]),
            self.grid.flat_index_on_extended_grid([i, j-1, k]),
            self.grid.flat_index_on_extended_grid([i, j, k-1])
        ];
        
        let u = 0.5 * (self.velocity[i_0][0] + self.velocity[i_n[0]][0]);
        let v = 0.5 * (self.velocity[i_0][1] + self.velocity[i_n[1]][1]);
        let w = 0.5 * (self.velocity[i_0][2] + self.velocity[i_n[2]][2]);
        
        SpatialVector([u, v, w])
    }
    
    /// Computes the union of the signed distance functions in geometries
    pub fn signed_distance_function(&self, point: SpatialVector) -> Float {
        let mut value = Float::MAX;
        
        for geometry in &self.geometries {
            let local_value = geometry.signed_distance(point);
            
            if local_value < value {
                value = local_value;
            }
        }
        
        value
    }
    
    pub fn correct_velocities_for_geometry(&mut self) {
        let start_time = Instant::now();
        
        let [nx, ny, nz] = self.grid.interior_shape;
        
        let mut max_dx = 0.0;
        for axis_index in 0..3 {
            if self.grid.cell_length[axis_index] > max_dx {
                max_dx = self.grid.cell_length[axis_index];
            }
        }
        
        let epsilon = 2.0 * max_dx;
        
        let nr_cells_interior = nx * ny * nz;

        let velocity_ptr = self.velocity.as_mut_ptr() as usize;
        let velocity_star_ptr = self.velocity_star.as_mut_ptr() as usize;

        (0..nr_cells_interior)
            .into_par_iter()
            .for_each(|i_flat_interior| {
                let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
                let [i, j, k] = self.grid.extended_indices_from_interior_indices(interior_indices);
                let i_0 = self.grid.flat_index_on_extended_grid([i, j, k]);
                
                let cell_center = self.grid.cell_center(interior_indices);

                let mut new_velocity = SpatialVector::default();
                let mut new_velocity_star = SpatialVector::default();
                
                for axis_index in 0..3 {
                    let mut face = cell_center;
                    face[axis_index] += 0.5 * self.grid.cell_length[axis_index];
                    
                    let sdf = self.signed_distance_function(face);
                    
                    let mu = blending_function(sdf, epsilon);
                    
                    new_velocity[axis_index] = mu * self.velocity[i_0][axis_index] + (1.0 - mu) * 1e-6;
                    new_velocity_star[axis_index] = mu * self.velocity_star[i_0][axis_index] + (1.0 - mu) * 1e-6;
                }

                unsafe {
                    let ptr = velocity_ptr as *mut SpatialVector;
                    *ptr.add(i_0) = new_velocity;
                }

                unsafe {
                    let ptr = velocity_star_ptr as *mut SpatialVector;
                    *ptr.add(i_0) = new_velocity_star;
                }
            });

        println!("Correct velocities for geometry time: {:.?}", start_time.elapsed());
    }
    
    pub fn actuator_line_initialization(&mut self) {
        if let Some(actuator_line) = self.actuator_line.as_mut() {
            let [nx, ny, nz] = self.grid.interior_shape;
            let nr_interior_cells = nx * ny * nz;
    
            // Collect results in parallel
            let results: Vec<_> = (0..nr_interior_cells)
                .into_par_iter()
                .map(|i_flat_interior| {
                    let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
                    let cell_center = self.grid.cell_center(interior_indices);
                    
                    let line_index = actuator_line.model.dominating_line_element_index_at_point(cell_center);
                    let projection_weight = actuator_line.model.summed_projection_weights_at_point(cell_center);
                    
                    (line_index, projection_weight)
                })
                .collect();
    
            // Unzip the results into the two vectors
            let (dominating_line_indices, summed_projection_weights): (Vec<_>, Vec<_>) = 
                results.into_iter().unzip();
            
            actuator_line.dominating_line_indices = dominating_line_indices;
            actuator_line.summed_projection_weights = summed_projection_weights;
        }
    }
    
    pub fn actuator_line_ctrl_points_velocity(&self) -> Vec<SpatialVector> {
        if let Some(actuator_line) = &self.actuator_line {
            let [nx, ny, nz] = self.grid.interior_shape;
            
            let nr_interior_cells = nx * ny * nz;
            
            let nr_span_lines = actuator_line.model.line_force_model.nr_span_lines();
            
            let mut numerator = vec![SpatialVector::default(); nr_span_lines];
            let mut denominator = vec![0.0; nr_span_lines];
            
            for i_flat_interior in 0..nr_interior_cells {
                let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
                
                let cell_center = self.grid.cell_center(interior_indices);
                
                let velocity = self.cell_center_velocity_from_interior_indices(interior_indices);
                
                let cell_volume = self.grid.cell_length[0] * self.grid.cell_length[1] * self.grid.cell_length[2];
                
                let line_index = actuator_line.dominating_line_indices[i_flat_interior];
                
                let (temp_num, temp_den) = actuator_line.model.get_weighted_velocity_sampling_integral_terms_for_cell(
                    line_index, 
                    velocity, 
                    cell_center, 
                    cell_volume
                );
                
                numerator[line_index] += temp_num;
                denominator[line_index] += temp_den;
            }
            
            let mut ctrl_points_velocity = vec![SpatialVector::default(); nr_span_lines];
            
            for line_index in 0..nr_span_lines {
                if denominator[line_index] != 0.0 {
                    ctrl_points_velocity[line_index] = numerator[line_index] / denominator[line_index];
                }
            }
            
            ctrl_points_velocity
        } else {
            vec![SpatialVector::default(); 1]
        }
    }
    
    pub fn run_actuator_line_model(&mut self, time: Float, time_step: Float) {
        let start_time = Instant::now();
        
        let ctrl_points_velocity = self.actuator_line_ctrl_points_velocity();
        
        // Step the model
        if let Some(actuator_line) = self.actuator_line.as_mut() {
            let nr_span_lines = actuator_line.model.line_force_model.nr_span_lines();
            
            for line_index in 0..nr_span_lines {
                actuator_line.model.ctrl_points_velocity[line_index] = ctrl_points_velocity[line_index]
            }
            
            actuator_line.model.do_step(time, time_step);
        }
        
        // Transfer body forces to the grid
        if let Some(actuator_line) = &self.actuator_line {
            let [nx, ny, nz] = self.grid.interior_shape;
            
            let nr_interior_cells = nx * ny * nz;
            
            let new_body_forces: Vec<(usize, SpatialVector)> = (0..nr_interior_cells)
                .into_par_iter()
                .map(|i_flat_interior| {
                    let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
                    let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
                    let i_flat_extended = self.grid.flat_index_on_extended_grid(extended_indices);
                    
                    let cell_velocity = self.cell_center_velocity_from_interior_indices(interior_indices);
                    
                    let line_index = actuator_line.dominating_line_indices[i_flat_interior];
                    
                    let body_force_weight = actuator_line.summed_projection_weights[i_flat_interior];
                    
                    let force_to_project = actuator_line.model.force_to_project_at_cell(
                        line_index, 
                        cell_velocity
                    );
                
                    (i_flat_extended, body_force_weight * force_to_project / self.density)
                }).collect();
            
            for (i_flat_extended, force) in new_body_forces {
                self.body_force[i_flat_extended] = force;
            }
            
        }

        println!("Running actuator line model: {:.?}", start_time.elapsed());
    }
    
    pub fn pressure_projection_rhs(&mut self, time_step: Float) {
        let start_time = Instant::now();
        
        let [nx, ny, nz] = self.grid.interior_shape;
        let [dx, dy, dz] = self.grid.cell_length.0;
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let interior_indices = [i_x, i_y, i_z];
                    
                    let i_0_int = self.grid.flat_index_on_interior_grid(interior_indices);
                    
                    let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
                    
                    let i_l = self.grid.local_flat_indices_on_extended_grid(extended_indices);
                    
                    let du_dx = (self.velocity_star[i_l.current][0] - self.velocity_star[i_l.neg[0]][0]) / dx;
                    let dv_dy = (self.velocity_star[i_l.current][1] - self.velocity_star[i_l.neg[1]][1]) / dy;
                    let dw_dz = (self.velocity_star[i_l.current][2] - self.velocity_star[i_l.neg[2]][2]) / dz;
                    
                    self.pressure_solver.rhs_at_levels[0][i_0_int] = self.density * (du_dx + dv_dy + dw_dz) / time_step;
                }
            }
        }

        println!("Pressure projection rhs time: {:.?}", start_time.elapsed());
    }
    
    pub fn project_pressure(&mut self, time_step: Float) {
        let start_time = Instant::now();
        self.pressure_projection_rhs(time_step);
        
        self.pressure_solver.solve();

        println!("Project pressure time: {:.?}", start_time.elapsed());
    }
    
    pub fn convect_and_diffuse(&mut self, time_step: Float) {
        let start_time = Instant::now();
    
        // Destructure borrows up front so the parallel closure can hold an
        // exclusive borrow of `velocity_star` alongside shared borrows of the
        // read-only fields, without capturing `&self` as a whole.
        let grid = &self.grid;
        let velocity = &self.velocity;
        let velocity_org = &self.velocity_org;
        let body_force = &self.body_force;
        let viscosity = self.viscosity;
        let density = self.density;
        let velocity_star = &mut self.velocity_star;
    
        let [nxi, nyi, nzi] = grid.interior_shape;
        let [_nx, ny, nz] = grid.extended_shape;
        let plane = ny * nz; // one full i-plane on the extended grid, contiguous
    
        // Parallelize over interior i-planes. Each task owns one contiguous
        // i-plane of `velocity_star` exclusively → no unsafe, no data races.
        // `skip(1).take(nxi)` selects interior planes, assuming a 1-cell halo.
        velocity_star
            .par_chunks_mut(plane)
            .enumerate()
            .skip(1)
            .take(nxi)
            .for_each(|(i, star_plane)| {
                let ii = i - 1; // interior i index
                for ji in 0..nyi {
                    let j = ji + 1;
                    // Interior flat index for (ii, ji, 0); advanced by +1 per k.
                    let mut flat_interior = grid.flat_index_on_interior_grid([ii, ji, 0]);
                    // Extended flat index for (i, j, 1); advanced by +1 per k.
                    let mut i_extended = grid.flat_index_on_extended_grid([i, j, 1]);
    
                    for _k in 0..nzi {
                        let convective_term = kernels::convective_term(flat_interior, grid, velocity);
                        let viscous_term    = kernels::viscous_term(flat_interior, grid, velocity, viscosity);
                        let body_force_term = kernels::body_force_term(flat_interior, grid, body_force, density);
    
                        let org = velocity_org[i_extended];
                        let new_velocity = SpatialVector([
                            org[0] + (convective_term[0] + viscous_term[0] - body_force_term[0]) * time_step,
                            org[1] + (convective_term[1] + viscous_term[1] - body_force_term[1]) * time_step,
                            org[2] + (convective_term[2] + viscous_term[2] - body_force_term[2]) * time_step,
                        ]);
    
                        // i_extended lies in plane `i`; index within the chunk.
                        star_plane[i_extended - i * plane] = new_velocity;
    
                        flat_interior += 1;
                        i_extended += 1;
                    }
                }
            });
    
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid,
            &mut self.velocity_star,
        );
    
        println!("Convect and diffuse time: {:.?}", start_time.elapsed());
    }
    
    pub fn update_velocity(&mut self, time_step: Float) {
        let start_time = Instant::now();
        
        let [nx, ny, nz] = self.grid.interior_shape;
        
        let nr_interior_cells = nx * ny * nz;
        
        let [dx, dy, dz] = self.grid.cell_length.0;

        let data_ptr = self.velocity.as_mut_ptr() as usize;

        let pressure = &self.pressure_solver.x_at_levels[0];

        (0..nr_interior_cells)
            .into_par_iter()
            .for_each(|i_flat| {
                let interior_indices = self.grid.interior_indices_from_flat_index(i_flat);
                let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
                
                let i_l = self.grid.local_flat_indices_on_extended_grid(extended_indices);

                let dp_dx = SpatialVector(
                    [
                        (pressure[i_l.pos[0]] - pressure[i_l.current]) / dx,
                        (pressure[i_l.pos[1]] - pressure[i_l.current]) / dy,
                        (pressure[i_l.pos[2]] - pressure[i_l.current]) / dz
                    ]
                );

                let new_velocity = self.velocity_star[i_l.current] - (time_step / self.density) * dp_dx;

                unsafe {
                    let ptr = data_ptr as *mut SpatialVector;
                    *ptr.add(i_l.current) = new_velocity;
                }
            });
        
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid,
            &mut self.velocity
        );

        println!("Update velocity time: {:.?}", start_time.elapsed());
    }
}
