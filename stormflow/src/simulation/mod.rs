use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;
use stormath::matrix::linalg::IterativeSolverSettings;
use stormath::spatial_vector::SpatialVector;

pub mod io;
pub mod builder;

use crate::grid::Grid;

use crate::boundary_conditions::{BoundaryCondition, BoundaryConditions};
use crate::actuator_line_interface::ActuatorLineInterface;

use rayon::prelude::*;

const MATRIX_ROW_LENGTH: usize = 9; 


pub struct Simulation {
    pub pressure: Vec<Float>,
    pub velocity_x: Vec<Float>,
    pub velocity_y: Vec<Float>,
    pub velocity_z: Vec<Float>,
    pub body_force: Vec<SpatialVector>,
    pub boundary_conditions: BoundaryConditions,
    pub pressure_matrix: SparseMatrix<MATRIX_ROW_LENGTH>,
    pub pressure_rhs_fixed: Vec<Float>,
    pub grid: Grid,
    pub viscosity: Float,
    pub density: Float,
    pub solver_settings: IterativeSolverSettings,
    pub actuator_line: Option<ActuatorLineInterface>
}

impl Simulation {
    pub fn initialize_after_build(&mut self) {
        println!("Initializing after build");
        self.set_fixed_pressure_system();
        self.actuator_line_initialization();
        println!();
    }

    pub fn do_step(&mut self, time: Float, time_step: Float) {
        println!("Setting velocity ghost cells");
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid,
            &mut self.velocity_x,
            &mut self.velocity_y,
            &mut self.velocity_z
        );
        
        println!("Computing velocity star");
        let mut u_star = self.velocity_star(time_step, &self.velocity_x, 0);
        let mut v_star = self.velocity_star(time_step, &self.velocity_y, 1);
        let mut w_star = self.velocity_star(time_step, &self.velocity_z, 2);
        
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid, 
            &mut u_star, 
            &mut v_star, 
            &mut w_star
        );
        
        println!("Computing pressure right hand side");
        let pressure_rhs = self.pressure_projection_rhs(time_step, &u_star, &v_star, &w_star);
        
        let initial_guess = self.grid.interior_values_from_extended_values(&self.pressure);
        
        println!("Solving the pressure");
        let pressure_interior = self.pressure_matrix.solve_jacobi(
            &pressure_rhs, &initial_guess, &self.solver_settings
        ).unwrap();
        
        println!("Transferring interior values to the extended grid");
        self.grid.transfer_interior_values_to_extended_grid(&pressure_interior, &mut self.pressure);
        
        self.boundary_conditions.set_pressure_ghost_cells(&self.grid, &mut self.pressure);
        
        println!("Final velocity update");
        self.update_velocity(time_step, &u_star, &v_star, &w_star);
        
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid,
            &mut self.velocity_x,
            &mut self.velocity_y,
            &mut self.velocity_z
        );
        
        println!("Running actuator line model");
        self.run_actuator_line_model(time, time_step);
        
        if let Some(actuator_line) = &self.actuator_line {
            actuator_line.model.write_results("");
        }
        
        println!();
    }
    
    /// Computes the cell center velocity, by interpolating from the faces to the center
    pub fn cell_center_velocity_from_interior_indices(&self, interior_indices: [usize; 3]) -> SpatialVector {
        let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
        
        let local_indices = self.grid.local_flat_indices_on_extended_grid(extended_indices);
        
        let u = 0.5 * (self.velocity_x[local_indices.current] + self.velocity_x[local_indices.x_neg[0]]);
        let v = 0.5 * (self.velocity_y[local_indices.current] + self.velocity_y[local_indices.y_neg[0]]);
        let w = 0.5 * (self.velocity_z[local_indices.current] + self.velocity_z[local_indices.z_neg[0]]);
        
        SpatialVector([u, v, w])
    }
    
    pub fn actuator_line_initialization(&mut self) {
        if let Some(actuator_line) = self.actuator_line.as_mut() {
            let [nx, ny, nz] = self.grid.nr_interior_cells();
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
            let [nx, ny, nz] = self.grid.nr_interior_cells();
            
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
            let [nx, ny, nz] = self.grid.nr_interior_cells();
            
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
    }
    
    pub fn set_fixed_pressure_system(&mut self) {
        let [dx, dy, dz] = self.grid.cell_length.0;
       
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        
        let nr_interior_cells = nx * ny * nz;
        
        let mut matrix: SparseMatrix<MATRIX_ROW_LENGTH> = SparseMatrix::new_default(nr_interior_cells);
        let mut rhs: Vec<Float> = vec![0.0; nr_interior_cells];
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let i_l = self.grid.local_flat_indices_on_interior_grid([i_x, i_y, i_z]);
                    
                    if i_x == 0 {
                        match self.boundary_conditions.pressure[0][0] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{i-1} = p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{-p_i + p_{i+1}}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.x_pos[0]]] += 1.0 / dx.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{i-1} + p_i) = value
                                // p_{i-1} = 2 * value - p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{2 * value - p_i - 2 p_i + p_{i+1}}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.x_pos[0]]] += 1.0 / dx.powi(2);
                                
                                rhs[i_l.current] += -2.0 * value / dx.powi(2);
                            }
                        }
                    } else if i_x == nx - 1 {
                        match self.boundary_conditions.pressure[0][1] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{i+1} = p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{-p_i + p_{i-1}}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.x_neg[0]]] += 1.0 / dx.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{i+1} + p_i) = value
                                // p_{i+1} = 2 * value - p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{p_{i-1} - 2 p_i + 2 * value - p_i}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.x_neg[0]]] += 1.0 / dx.powi(2);
                                
                                rhs[i_l.current] += -2.0 * value / dx.powi(2);
                            }
                        }
                    } else {
                        matrix[[i_l.current, i_l.x_neg[0]]] += 1.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.x_pos[0]]] += 1.0 / dx.powi(2);
                    }

                    // Y direction
                    if i_y == 0 {
                        match self.boundary_conditions.pressure[1][0] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{j-1} = p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{-p_j + p_{j+1}}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.y_pos[0]]] += 1.0 / dy.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{j-1} + p_j) = value
                                // p_{j-1} = 2 * value - p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{2 * value - p_j - 2 p_j + p_{j+1}}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.y_pos[0]]] += 1.0 / dy.powi(2);

                                rhs[i_l.current] += -2.0 * value / dy.powi(2);
                            }
                        }
                    } else if i_y == ny - 1 {
                        match self.boundary_conditions.pressure[1][1] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{j+1} = p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{-p_j + p_{j-1}}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.y_neg[0]]] += 1.0 / dy.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{j+1} + p_j) = value
                                // p_{j+1} = 2 * value - p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{p_{j-1} - 2 p_j + 2 * value - p_j}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.y_neg[0]]] += 1.0 / dy.powi(2);

                                rhs[i_l.current] += -2.0 * value / dy.powi(2);
                            }
                        }
                    } else {
                        matrix[[i_l.current, i_l.y_neg[0]]] += 1.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.y_pos[0]]] += 1.0 / dy.powi(2);
                    }

                    // Z direction
                    if i_z == 0 {
                        match self.boundary_conditions.pressure[2][0] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{k-1} = p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{-p_k + p_{k+1}}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.z_pos[0]]] += 1.0 / dz.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{k-1} + p_k) = value
                                // p_{k-1} = 2 * value - p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{2 * value - p_k - 2 p_k + p_{k+1}}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.z_pos[0]]] += 1.0 / dz.powi(2);

                                rhs[i_l.current] += -2.0 * value / dz.powi(2);
                            }
                        }
                    } else if i_z == nz - 1 {
                        match self.boundary_conditions.pressure[2][1] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{k+1} = p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{-p_k + p_{k-1}}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.z_neg[0]]] += 1.0 / dz.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{k+1} + p_k) = value
                                // p_{k+1} = 2 * value - p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{p_{k-1} - 2 p_k + 2 * value - p_k}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.z_neg[0]]] += 1.0 / dz.powi(2);

                                rhs[i_l.current] += -2.0 * value / dz.powi(2);
                            }
                        }
                    } else {
                        matrix[[i_l.current, i_l.z_neg[0]]] += 1.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.z_pos[0]]] += 1.0 / dz.powi(2);
                    }
                }
            }
        }
        
        self.pressure_matrix = matrix;
        self.pressure_rhs_fixed = rhs;
    }
    
    pub fn pressure_projection_rhs(
        &self,
        time_step: Float,
        u_star: &[Float],
        v_star: &[Float], 
        w_star: &[Float]
    ) -> Vec<Float> {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        let [dx, dy, dz] = self.grid.cell_length.0;
        
        let nr_interior_cells = nx * ny * nz;
        
        let mut out = vec![0.0; nr_interior_cells];
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let interior_indices = [i_x, i_y, i_z];
                    
                    let i_0_int = self.grid.flat_index_on_interior_grid(interior_indices);
                    
                    let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
                    
                    let i_l = self.grid.local_flat_indices_on_extended_grid(extended_indices);
                    
                    let du_dx = (u_star[i_l.current] - u_star[i_l.x_neg[0]]) / dx;
                    let dv_dy = (v_star[i_l.current] - v_star[i_l.y_neg[0]]) / dy;
                    let dw_dz = (w_star[i_l.current] - w_star[i_l.z_neg[0]]) / dz;
                    
                    out[i_0_int] = self.pressure_rhs_fixed[i_0_int] + 
                        self.density * (du_dx + dv_dy + dw_dz) / time_step;
                }
            }
        }
        
        out
    }
    
    
    
    /// Function that computes the updated velocity, minus the pressure gradient, for the supplied 
    /// velocity component vector, phi
    pub fn velocity_star(&self, time_step: Float, phi: &[Float], axis_index: usize) -> Vec<Float>{
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        
        let nr_cells = nx * ny * nz;
        
        let [dx, dy, dz] = self.grid.cell_length.0;
        
        let mut phi_star = phi.to_vec();
        
        for i_flat in 0..nr_cells {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat);
            let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
            
            let i_l = self.grid.local_flat_indices_on_extended_grid(extended_indices);
            
            let mut convective_term = 0.0;
            
            let phi_x_neg = 0.5 * (phi[i_l.current] + phi[i_l.x_neg[0]]);
            let phi_x_pos = 0.5 * (phi[i_l.current] + phi[i_l.x_pos[0]]);                    
            let u_x_neg = 0.5 * (self.velocity_x[i_l.current] + self.velocity_x[i_l.x_neg[0]]);
            let u_x_pos = 0.5 * (self.velocity_x[i_l.current] + self.velocity_x[i_l.x_pos[0]]);
            
            convective_term -= (phi_x_pos * u_x_pos - phi_x_neg * u_x_neg)/dx;
            
            let phi_y_neg = 0.5 * (phi[i_l.current] + phi[i_l.y_neg[0]]);
            let phi_y_pos = 0.5 * (phi[i_l.current] + phi[i_l.y_pos[0]]);
            
            let v_y_neg = 0.5 * (self.velocity_y[i_l.current] + self.velocity_y[i_l.y_neg[0]]);
            let v_y_pos = 0.5 * (self.velocity_y[i_l.current] + self.velocity_y[i_l.y_pos[0]]);
            
            convective_term -= (phi_y_pos * v_y_pos - phi_y_neg * v_y_neg)/dy;
            
            let phi_z_neg = 0.5 * (phi[i_l.current] + phi[i_l.z_neg[0]]);
            let phi_z_pos = 0.5 * (phi[i_l.current] + phi[i_l.z_pos[0]]);
            
            let w_z_neg = 0.5 * (self.velocity_z[i_l.current] + self.velocity_z[i_l.z_neg[0]]);
            let w_z_pos = 0.5 * (self.velocity_z[i_l.current] + self.velocity_z[i_l.z_pos[0]]);
            
            convective_term -= (phi_z_pos * w_z_pos - phi_z_neg * w_z_neg)/dz;
            
            // viscous term
            let d2phi_dx2 = (
                phi[i_l.x_pos[0]] - 
                2.0 * phi[i_l.current] + 
                phi[i_l.x_neg[0]]
            ) / dx.powi(2);
            
            let d2phi_dy2 = (
                phi[i_l.y_pos[0]] - 
                2.0 * phi[i_l.current] + 
                phi[i_l.y_neg[0]]
            ) / dy.powi(2);
            
            let d2phi_dz2 = (
                phi[i_l.z_pos[0]] - 
                2.0 * phi[i_l.current] + 
                phi[i_l.z_neg[0]]
            ) / dz.powi(2);
            
            let viscous_term = self.viscosity * (d2phi_dx2 + d2phi_dy2 + d2phi_dz2);
            
            // Body force
            let body_force = match axis_index {
                0 => 0.5 * (self.body_force[i_l.current][0] + self.body_force[i_l.x_neg[0]][0]) / self.density,
                1 => 0.5 * (self.body_force[i_l.current][1] + self.body_force[i_l.y_neg[0]][1]) / self.density,
                2 => 0.5 * (self.body_force[i_l.current][2] + self.body_force[i_l.z_neg[0]][2]) / self.density,
                _ => panic!("Axis index larger than 2, which does not make sense")
            };
            
            let du_dt = convective_term + viscous_term - body_force; // TODO: decide more on the sign...
            
            phi_star[i_l.current] = phi[i_l.current] + du_dt * time_step;
        }
        
        phi_star
    }
    
    pub fn update_velocity(&mut self, time_step: Float, u_star: &[Float], v_star: &[Float], w_star: &[Float]) {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        
        let nr_cells = nx * ny * nz;
        
        let [dx, dy, dz] = self.grid.cell_length.0;
        
        for i_flat in 0..nr_cells {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat);
            let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
            
            let i_l = self.grid.local_flat_indices_on_extended_grid(extended_indices);
            
            let dp_dx = (self.pressure[i_l.x_pos[0]] - self.pressure[i_l.current]) / dx;
            let dp_dy = (self.pressure[i_l.y_pos[0]] - self.pressure[i_l.current]) / dy;
            let dp_dz = (self.pressure[i_l.z_pos[0]] - self.pressure[i_l.current]) / dz;
            
            self.velocity_x[i_l.current] = u_star[i_l.current]  - (time_step / self.density) * dp_dx;
            self.velocity_y[i_l.current] = v_star[i_l.current]  - (time_step / self.density) * dp_dy;
            self.velocity_z[i_l.current] = w_star[i_l.current]  - (time_step / self.density) * dp_dz;
        }
    }
}
