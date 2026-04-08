use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;
use stormath::matrix::linalg::IterativeSolverSettings;
use stormath::spatial_vector::SpatialVector;

pub mod io;
pub mod builder;

use crate::grid::Grid;

use crate::boundary_conditions::{BoundaryConditions};
use crate::actuator_line_interface::ActuatorLineInterface;
use crate::staggered_spatial_vectors::StaggeredSpatialVectors;

use rayon::prelude::*;

const MATRIX_ROW_LENGTH: usize = 9;


pub struct Simulation {
    pub pressure: Vec<Float>,
    pub velocity: StaggeredSpatialVectors,
    pub body_force: Vec<SpatialVector>,
    pub boundary_conditions: BoundaryConditions,
    pub pressure_matrix: SparseMatrix<MATRIX_ROW_LENGTH>,
    pub grid: Grid,
    pub viscosity: Float,
    pub density: Float,
    pub solver_settings: IterativeSolverSettings,
    pub actuator_line: Option<ActuatorLineInterface>,
}

impl Simulation {
    pub fn initialize_after_build(&mut self) {
        println!("Initializing after build");
        self.set_fixed_pressure_system();
        self.actuator_line_initialization();
        println!();
    }

    pub fn do_step(&mut self, time: Float, time_step: Float) {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        
        let nr_interior_cells = nx * ny * nz;
        let nr_extended_cells = self.grid.nr_extended_cells[0] * 
            self.grid.nr_extended_cells[1] * 
            self.grid.nr_extended_cells[2];
        
        println!("Setting velocity ghost cells");
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid,
            &mut self.velocity
        );
        
        println!("Computing velocity star");
        let convective_term = self.convective_term();
        let viscous_term = self.viscous_term();
        let pressure_gradient_term = self.pressure_gradient_term();
        let body_force_term = self.body_force_term();
        
        let mut velocity_star = StaggeredSpatialVectors::new_default(nr_extended_cells);
        
        for a_i in 0..3 {
            for i in 0..nr_extended_cells {
                
                velocity_star.data[a_i][i] = self.velocity.data[a_i][i] + (
                    convective_term.data[a_i][i] + 
                    viscous_term.data[a_i][i] - 
                    pressure_gradient_term.data[a_i][i] - 
                    body_force_term.data[a_i][i]
                ) * time_step;
            }
        }
        
         self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid, 
            &mut velocity_star
        );
        
        println!("Computing pressure right hand side");
        let pressure_rhs = self.pressure_projection_rhs(
            time_step, 
            &velocity_star.data[0], 
            &velocity_star.data[1], 
            &velocity_star.data[2]
        );
        
        //let initial_guess = self.grid.interior_values_from_extended_values(&self.pressure);
        
        let initial_guess = vec![0.0; nr_interior_cells];
        
        println!("Solving the pressure");
        let pressure_delta_interior = self.pressure_matrix.solve_jacobi(
            &pressure_rhs, &initial_guess, &self.solver_settings
        ).unwrap();
        
        println!("Transferring interior values to the extended grid");
        
        let mut pressure_delta_extended = vec![0.0; nr_extended_cells];
        
        for i_flat_i in 0..nr_interior_cells {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_i);
            let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
            let i_flat_e = self.grid.flat_index_on_extended_grid(extended_indices);
            
            self.pressure[i_flat_e] += pressure_delta_interior[i_flat_i];
            pressure_delta_extended[i_flat_e] = pressure_delta_interior[i_flat_i];
        }
        
        BoundaryConditions::set_pressure_ghost_cells_to_zero_gradient(&self.grid, &mut pressure_delta_extended);
        
        self.boundary_conditions.set_pressure_ghost_cells(&self.grid, &mut self.pressure);
        
        println!("Final velocity update");
        self.update_velocity(
            time_step, 
            &velocity_star,
            &pressure_delta_extended
        );
        
        self.boundary_conditions.set_velocity_ghost_cells(
            &self.grid,
            &mut self.velocity
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
        let [i, j, k] = self.grid.extended_indices_from_interior_indices(interior_indices);
        
        let i_0 = self.grid.flat_index_on_extended_grid([i, j, k]);
        
        let i_n = [
            self.grid.flat_index_on_extended_grid([i-1, j, k]),
            self.grid.flat_index_on_extended_grid([i, j-1, k]),
            self.grid.flat_index_on_extended_grid([i, j, k-1])
        ];
        
        let u = 0.5 * (self.velocity[[0, i_0]] + self.velocity[[0, i_n[0]]]);
        let v = 0.5 * (self.velocity[[1, i_0]] + self.velocity[[1, i_n[1]]]);
        let w = 0.5 * (self.velocity[[2, i_0]] + self.velocity[[2, i_n[2]]]);
        
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
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let i_l = self.grid.local_flat_indices_on_interior_grid([i_x, i_y, i_z]);
                    
                    if i_x == 0 {
                        matrix[[i_l.current, i_l.current]] += -1.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.pos[0]]] += 1.0 / dx.powi(2);
                    } else if i_x == nx - 1 {
                        matrix[[i_l.current, i_l.current]] += -1.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.neg[0]]] += 1.0 / dx.powi(2);
                    } else {
                        matrix[[i_l.current, i_l.neg[0]]] += 1.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.pos[0]]] += 1.0 / dx.powi(2);
                    }

                    // Y direction
                    if i_y == 0 {
                        matrix[[i_l.current, i_l.current]] += -1.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.pos[1]]] += 1.0 / dy.powi(2);
                    } else if i_y == ny - 1 {
                        matrix[[i_l.current, i_l.current]] += -1.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.neg[1]]] += 1.0 / dy.powi(2);
                    } else {
                        matrix[[i_l.current, i_l.neg[1]]] += 1.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.pos[1]]] += 1.0 / dy.powi(2);
                    }

                    // Z direction
                    if i_z == 0 {
                        matrix[[i_l.current, i_l.current]] += -1.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.pos[2]]] += 1.0 / dz.powi(2);
                    } else if i_z == nz - 1 {
                        matrix[[i_l.current, i_l.current]] += -1.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.neg[2]]] += 1.0 / dz.powi(2);
                    } else {
                        matrix[[i_l.current, i_l.neg[2]]] += 1.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.pos[2]]] += 1.0 / dz.powi(2);
                    }
                }
            }
        }
        
        self.pressure_matrix = matrix;
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
                    
                    let du_dx = (u_star[i_l.current] - u_star[i_l.neg[0]]) / dx;
                    let dv_dy = (v_star[i_l.current] - v_star[i_l.neg[1]]) / dy;
                    let dw_dz = (w_star[i_l.current] - w_star[i_l.neg[2]]) / dz;
                    
                    out[i_0_int] = self.density * (du_dx + dv_dy + dw_dz) / time_step;
                }
            }
        }
        
        out
    }
    
    
    /// Calculates the convective term of the Navier Stokes equation suing finite difference.
    /// 
    /// Convective term, index notation:
    /// - u_j du_i/dx_j
    pub fn convective_term(&self) -> StaggeredSpatialVectors {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        let [nx_ext, ny_ext, nz_ext] = self.grid.nr_extended_cells.clone();
        
        let nr_cells_interior = nx * ny * nz;
        let nr_cells_extended = nx_ext * ny_ext * nz_ext;
        
        let mut out = StaggeredSpatialVectors::new_default(nr_cells_extended);
        
        for i_flat_interior in 0..nr_cells_interior {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
            
            let [i, j, k] = self.grid.extended_indices_from_interior_indices(interior_indices);
            
            let i_0 = self.grid.flat_index_on_extended_grid([i, j, k]);
            
            let i_p = [
                self.grid.flat_index_on_extended_grid([i+1, j, k]),
                self.grid.flat_index_on_extended_grid([i, j+1, k]),
                self.grid.flat_index_on_extended_grid([i, j, k+1])
            ];
            
            let i_n = [
                self.grid.flat_index_on_extended_grid([i-1, j, k]),
                self.grid.flat_index_on_extended_grid([i, j-1, k]),
                self.grid.flat_index_on_extended_grid([i, j, k-1])
            ];
            
            for a_i_1 in 0..3 {
                let mut indices_p_i = [i, j, k];
                indices_p_i[a_i_1] += 1; // Indices to neighbor cell relative to u_i
                
                let i_p_i = self.grid.flat_index_on_extended_grid(indices_p_i);
                
                for a_i_2 in 0..3 {
                    let u_j = if a_i_1 == a_i_2 {
                        self.velocity[[a_i_2, i_0]]
                    } else {
                        let mut indices_pn_i = indices_p_i.clone();
                        indices_pn_i[a_i_2] -= 1;
                        let i_pn = self.grid.flat_index_on_extended_grid(indices_pn_i);
                        
                        0.25 * (
                            self.velocity[[a_i_2, i_0]] + // Current cell
                            self.velocity[[a_i_2, i_p_i]] + // Neighbor, in the u_i direction
                            self.velocity[[a_i_2, i_n[a_i_2]]] + // Current cell, opposite face
                            self.velocity[[a_i_2, i_pn]] // Neighbor, in the u_i direction, opposite face
                        )
                    };
                    
                    let u_i_p = self.velocity[[a_i_1, i_p[a_i_2]]];
                    let u_i_n = self.velocity[[a_i_1, i_n[a_i_2]]];
                    
                    let dui_dxj = (u_i_p - u_i_n) / (2.0 * self.grid.cell_length[a_i_2]); 
                     
                    out[[a_i_1, i_0]] -= u_j * dui_dxj; 
                 }
            }
        }
        
        out
    }
    
    pub fn viscous_term(&self) -> StaggeredSpatialVectors {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        let [nx_ext, ny_ext, nz_ext] = self.grid.nr_extended_cells.clone();
        
        let nr_cells_interior = nx * ny * nz;
        let nr_cells_extended = nx_ext * ny_ext * nz_ext;
        
        let mut out = StaggeredSpatialVectors::new_default(nr_cells_extended);
        
        for i_flat_interior in 0..nr_cells_interior {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
            
            let [i, j, k] = self.grid.extended_indices_from_interior_indices(interior_indices);
            
            let i_0 = self.grid.flat_index_on_extended_grid([i, j, k]);
            
            for vel_comp in 0..3 {
                for deriv_dir in 0..3 {
                    let mut indices_p = [i, j, k];
                    indices_p[deriv_dir] += 1;
                    let i_p = self.grid.flat_index_on_extended_grid(indices_p);
                    
                    let mut indices_n = [i, j, k];
                    indices_n[deriv_dir] -= 1;
                    let i_n = self.grid.flat_index_on_extended_grid(indices_n);
                    
                    out[[vel_comp, i_0]] += self.viscosity * (
                        self.velocity[[vel_comp, i_p]] - 
                        2.0 * self.velocity[[vel_comp, i_0]] + 
                        self.velocity[[vel_comp, i_n]]
                    ) / self.grid.cell_length[deriv_dir].powi(2);
                }
            }
        }
        
        out
    }
    
    pub fn pressure_gradient_term(&self) -> StaggeredSpatialVectors {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        let [nx_ext, ny_ext, nz_ext] = self.grid.nr_extended_cells.clone();
        
        let nr_cells_interior = nx * ny * nz;
        let nr_cells_extended = nx_ext * ny_ext * nz_ext;
        
        let mut out = StaggeredSpatialVectors::new_default(nr_cells_extended);
        
        for i_flat_interior in 0..nr_cells_interior {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
            
            let [i, j, k] = self.grid.extended_indices_from_interior_indices(interior_indices);
            
            let i_0 = self.grid.flat_index_on_extended_grid([i, j, k]);
            
            for a_i in 0..3 {
                let mut indices_p = [i, j, k];
                indices_p[a_i] += 1;
                let i_p = self.grid.flat_index_on_extended_grid(indices_p);
                
                out[[a_i, i_0]] += (self.pressure[i_p] - self.pressure[i_0]) / 
                    (self.density * self.grid.cell_length[a_i]);
            }
        }
        
        out
    }
    
    pub fn body_force_term(&self) -> StaggeredSpatialVectors {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        let [nx_ext, ny_ext, nz_ext] = self.grid.nr_extended_cells.clone();
        
        let nr_cells_interior = nx * ny * nz;
        let nr_cells_extended = nx_ext * ny_ext * nz_ext;
        
        let mut out = StaggeredSpatialVectors::new_default(nr_cells_extended);
        
        for i_flat_interior in 0..nr_cells_interior {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat_interior);
            
            let [i, j, k] = self.grid.extended_indices_from_interior_indices(interior_indices);
            
            let i_0 = self.grid.flat_index_on_extended_grid([i, j, k]);
            
            for a_i in 0..3 {
                let mut indices_p = [i, j, k];
                indices_p[a_i] += 1;
                let i_p = self.grid.flat_index_on_extended_grid(indices_p);
                
                out[[a_i, i_0]] += 0.5 * (self.body_force[i_0][a_i] + self.body_force[i_p][a_i]) / self.density
            }
        }
        
        out
    }
    
    pub fn update_velocity(
        &mut self, 
        time_step: Float, 
        velocity_star: &StaggeredSpatialVectors, 
        delta_pressure: &[Float]
    ) {
        let [nx, ny, nz] = self.grid.nr_interior_cells();
        
        let nr_cells = nx * ny * nz;
        
        let [dx, dy, dz] = self.grid.cell_length.0;
        
        for i_flat in 0..nr_cells {
            let interior_indices = self.grid.interior_indices_from_flat_index(i_flat);
            let extended_indices = self.grid.extended_indices_from_interior_indices(interior_indices);
            
            let i_l = self.grid.local_flat_indices_on_extended_grid(extended_indices);
            
            let dp_dx = (delta_pressure[i_l.pos[0]] - delta_pressure[i_l.current]) / dx;
            let dp_dy = (delta_pressure[i_l.pos[1]] - delta_pressure[i_l.current]) / dy;
            let dp_dz = (delta_pressure[i_l.pos[2]] - delta_pressure[i_l.current]) / dz;
            
            self.velocity[[0, i_l.current]] = velocity_star.data[0][i_l.current]  - (time_step / self.density) * dp_dx;
            self.velocity[[1, i_l.current]] = velocity_star.data[1][i_l.current]  - (time_step / self.density) * dp_dy;
            self.velocity[[2, i_l.current]] = velocity_star.data[2][i_l.current]  - (time_step / self.density) * dp_dz;
        }
    }
}
