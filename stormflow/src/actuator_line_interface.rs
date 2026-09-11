use rayon::prelude::*;

use stormbird::actuator_line::ActuatorLine;
use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

use crate::grid::Grid;

pub struct ActuatorLineInterface {
    pub model: ActuatorLine,
    pub cell_indices_to_check: Vec<usize>,
    pub dominating_line_indices: Vec<usize>,
    pub dominating_line_weights: Vec<Float>,
}

impl ActuatorLineInterface {
    pub fn new(model: ActuatorLine, grid: &Grid) -> Self {
        let nr_interior_cells = grid.nr_interior_cells();

        let results: Vec<_> = (0..nr_interior_cells)
            .into_par_iter()
            .map(|i_flat_interior| {
                let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
                let cell_center = grid.cell_center(interior_indices);

                let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
                let flat_index = grid.flat_index_on_extended_grid(extended_indices);
                
                let (line_index, projection_weight) = model.dominating_line_element_and_weight_at_point(cell_center);

                (flat_index, line_index, projection_weight)
            })
            .collect();

        let mut cell_indices_to_check = Vec::new();
        let mut dominating_line_indices = Vec::new();
        let mut dominating_line_weights = Vec::new();

        for i in 0..results.len() {
            let (flat_index, line_index, projection_weight) = results[i];

            if projection_weight > model.sampling_settings.weight_limit {
                cell_indices_to_check.push(flat_index);
                dominating_line_indices.push(line_index);
                dominating_line_weights.push(projection_weight);
            }
        }

        println!(
            "Number of cells with actuator line interaction: {:.?}", 
            cell_indices_to_check.len()
        );

        
        Self {
            model,
            cell_indices_to_check,
            dominating_line_indices,
            dominating_line_weights
        }
    }

    pub fn update_ctrl_points_velocity(
        &mut self, 
        grid: &Grid, 
        velocity: &[SpatialVector]
    ) {           
        let nr_cells_to_check = self.cell_indices_to_check.len();
        
        let nr_span_lines = self.model.line_force_model.nr_span_lines();
        
        let mut numerator = vec![SpatialVector::default(); nr_span_lines];
        let mut denominator = vec![0.0; nr_span_lines];

        let cell_volume = grid.cell_length[0] * grid.cell_length[1] * grid.cell_length[2];
        
        for i in 0..nr_cells_to_check {
            let i_flat_extended = self.cell_indices_to_check[i];
            
            let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);
            let interior_indices = grid.interior_indices_from_extended_indices(extended_indices);
            
            let cell_center = grid.cell_center(interior_indices);
            
            let velocity = grid.cell_centered_value_from_face_staggered(
                interior_indices, 
                velocity
            );
            
            let line_index = self.dominating_line_indices[i];
            
            let (temp_num, temp_den) = self.model.get_weighted_velocity_sampling_integral_terms_for_cell(
                line_index, 
                velocity, 
                cell_center, 
                cell_volume
            );
            
            numerator[line_index] += temp_num;
            denominator[line_index] += temp_den;
        }
        
        for line_index in 0..nr_span_lines {
            if denominator[line_index] != 0.0 {
                self.model.ctrl_points_velocity[line_index] = numerator[line_index] / denominator[line_index];
            }
        }
    }

    pub fn step_model(&mut self, time: Float, time_step: Float, grid: &Grid, velocity: &[SpatialVector]) {
        self.update_ctrl_points_velocity(grid, velocity);

        self.model.do_step(time, time_step);
    }

    pub fn compute_body_force(
        &self, 
        grid: &Grid, 
        velocity: &[SpatialVector], 
        density: Float,
        body_force: &mut [SpatialVector]
    ) {
        let nr_cells_to_check = self.cell_indices_to_check.len();
        
        let new_body_forces: Vec<(usize, SpatialVector)> = (0..nr_cells_to_check)
            .into_par_iter()
            .map(|i| {
                let i_flat_extended = self.cell_indices_to_check[i];
                let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);
                let interior_indices = grid.interior_indices_from_extended_indices(extended_indices);
                
                let cell_velocity = grid.cell_centered_value_from_face_staggered(
                    interior_indices, 
                    velocity
                );
                
                let line_index = self.dominating_line_indices[i];
                
                let body_force_weight = self.dominating_line_weights[i];
                
                let line_force_force = self.model.force_to_project_at_cell(
                    line_index, 
                    cell_velocity
                );

                let spanwise_damping_force = self.model.spanwise_damping_flow(
                    line_index,
                    cell_velocity
                );

                let force_to_project = line_force_force + spanwise_damping_force;
            
                (i_flat_extended, body_force_weight * force_to_project / density)
            }).collect();
        
        for (i_flat_extended, force) in new_body_forces {
            body_force[i_flat_extended] = -force;
        }
    }
}
