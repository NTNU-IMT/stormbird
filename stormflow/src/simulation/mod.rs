use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

pub mod io;
pub mod builder;
pub mod kernels;

use crate::grid::Grid;

use crate::boundary_conditions::{
    velocity::VelocityBoundaryConditions
};
use crate::actuator_line_interface::ActuatorLineInterface;
use crate::geometry::{
    Geometry,
};

use crate::pressure_solver::PressureSolverMultiGrid;

use rayon::prelude::*;

use std::time::Instant;

pub struct Simulation {
    pub velocity: Vec<SpatialVector>,
    pub velocity_org: Vec<SpatialVector>,
    pub velocity_star: Vec<SpatialVector>,
    pub velocity_boundary_conditions: VelocityBoundaryConditions,
    pub body_force: Vec<SpatialVector>,
    pub pressure_solver: PressureSolverMultiGrid,
    pub grid: Grid,
    pub viscosity: Float,
    pub density: Float,
    pub signed_distance_function: Vec<Float>,
    pub actuator_line: Option<ActuatorLineInterface>,
}

impl Simulation {
    pub fn initialize_after_build(&mut self) {
        println!("Initializing after build");
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
        self.velocity_boundary_conditions.set_ghost_cells(
            &self.grid,
            &mut self.velocity
        );

        self.correct_velocities_for_geometry();

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
                let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
                let i_0 = self.grid.flat_index_on_extended_grid(extended_indices);

                let mut new_velocity = SpatialVector::default();
                let mut new_velocity_star = SpatialVector::default();
                
                for axis_index in 0..3 {
                    let mut extended_indices_p = extended_indices;
                    extended_indices_p[axis_index] += 1;

                    let i_p = self.grid.flat_index_on_extended_grid(extended_indices_p);
                    
                    let sdf = 0.5 * (
                        self.signed_distance_function[i_0] + 
                        self.signed_distance_function[i_p]
                    );
                    
                    let mu = Geometry::blending_function(sdf, epsilon);
                    
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
    
    pub fn actuator_line_ctrl_points_velocity(&self) -> Vec<SpatialVector> {
        if let Some(actuator_line) = &self.actuator_line {            
            let nr_cells_to_check = actuator_line.cell_indices_to_check.len();
            
            let nr_span_lines = actuator_line.model.line_force_model.nr_span_lines();
            
            let mut numerator = vec![SpatialVector::default(); nr_span_lines];
            let mut denominator = vec![0.0; nr_span_lines];

            let cell_volume = self.grid.cell_length[0] * self.grid.cell_length[1] * self.grid.cell_length[2];
            
            for i in 0..nr_cells_to_check {
                let i_flat_extended = actuator_line.cell_indices_to_check[i];
                
                let extended_indices = self.grid.extended_indices_from_flat_index(i_flat_extended);
                let interior_indices = self.grid.interior_indices_from_extended_indices(extended_indices);
                
                let cell_center = self.grid.cell_center(interior_indices);
                
                let velocity = self.cell_center_velocity_from_interior_indices(interior_indices);
                
                let line_index = actuator_line.dominating_line_indices[i];
                
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
            let nr_cells_to_check = actuator_line.cell_indices_to_check.len();
            
            let new_body_forces: Vec<(usize, SpatialVector)> = (0..nr_cells_to_check)
                .into_par_iter()
                .map(|i| {
                    let i_flat_extended = actuator_line.cell_indices_to_check[i];
                    let extended_indices = self.grid.extended_indices_from_flat_index(i_flat_extended);
                    let interior_indices = self.grid.interior_indices_from_extended_indices(extended_indices);
                    
                    let cell_velocity = self.cell_center_velocity_from_interior_indices(interior_indices);
                    
                    let line_index = actuator_line.dominating_line_indices[i];
                    
                    let body_force_weight = actuator_line.summed_projection_weights[i];
                    
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
        
        let nr_interior_cells = self.grid.nr_interior_cells();

        let data_ptr = self.pressure_solver.rhs_at_levels[0].as_mut_ptr() as usize;

        (0..nr_interior_cells)
            .into_par_iter()
            .for_each(|i_flat_interior| {
                let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
                let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
                let i_0 = self.grid.flat_index_on_extended_grid(extended_indices);

                let mut new_value = 0.0;
                
                for axis_index in 0..3 {
                    let mut extended_indices_n = extended_indices;

                    extended_indices_n[axis_index] -= 1;
                    
                    let i_n = self.grid.flat_index_on_extended_grid(extended_indices_n);

                    new_value += (
                        self.velocity_star[i_0][axis_index] - 
                        self.velocity_star[i_n][axis_index]
                    ) / self.grid.cell_length[axis_index];
                }

                new_value *= self.density / time_step;

                unsafe {
                    let ptr = data_ptr as *mut Float;
                    *ptr.add(i_flat_interior) = new_value;
                }
                
            });

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
                    let mut i_interior = grid.flat_index_on_interior_grid([ii, ji, 0]);
                    let mut i_extended = grid.flat_index_on_extended_grid([i, j, 1]);
    
                    for _k in 0..nzi {
                        let new_value = kernels::convect_and_diffuse(
                            i_interior, 
                            grid, 
                            velocity,
                            body_force,
                            viscosity,
                            density
                        );
    
                        let new_velocity = velocity_org[i_extended] + time_step * new_value;
    
                        // i_extended lies in plane `i`; index within the chunk.
                        star_plane[i_extended - i * plane] = new_velocity;
    
                        i_interior += 1;
                        i_extended += 1;
                    }
                }
            });
    
        self.velocity_boundary_conditions.set_ghost_cells(
            &self.grid,
            &mut self.velocity_star,
        );
    
        println!("Convect and diffuse time: {:.?}", start_time.elapsed());
    }
    
    pub fn update_velocity(&mut self, time_step: Float) {
        let start_time = Instant::now();
        
        let nr_interior_cells = self.grid.nr_interior_cells();

        let data_ptr = self.velocity.as_mut_ptr() as usize;

        let pressure = &self.pressure_solver.x_at_levels[0];

        (0..nr_interior_cells)
            .into_par_iter()
            .for_each(|i_flat| {
                let interior_indices = self.grid.interior_indices_from_flat_index(i_flat);
                let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);

                let i_0 = self.grid.flat_index_on_extended_grid(extended_indices);

                let mut dp_dx = SpatialVector::default();

                for axis_index in 0..3 {
                    let mut extended_indices_p = extended_indices;
                    extended_indices_p[axis_index] += 1;

                    let i_p = self.grid.flat_index_on_extended_grid(extended_indices_p);

                    dp_dx[axis_index] = (
                        pressure[i_p] - 
                        pressure[i_0]
                    ) / self.grid.cell_length[axis_index];
                }
                
                let new_velocity = self.velocity_star[i_0] - (time_step / self.density) * dp_dx;

                unsafe {
                    let ptr = data_ptr as *mut SpatialVector;
                    *ptr.add(i_0) = new_velocity;
                }
            });
        
        self.velocity_boundary_conditions.set_ghost_cells(
            &self.grid,
            &mut self.velocity
        );

        println!("Update velocity time: {:.?}", start_time.elapsed());
    }
}