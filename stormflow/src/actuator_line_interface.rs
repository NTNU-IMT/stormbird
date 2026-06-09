use rayon::prelude::*;

use stormbird::actuator_line::ActuatorLine;
use stormath::type_aliases::Float;

use crate::grid::Grid;

pub struct ActuatorLineInterface {
    pub model: ActuatorLine,
    pub cell_indices_to_check: Vec<usize>,
    pub dominating_line_indices: Vec<usize>,
    pub summed_projection_weights: Vec<Float>,
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
                
                let line_index = model.dominating_line_element_index_at_point(cell_center);
                let projection_weight = model.summed_projection_weights_at_point(cell_center);
                
                (flat_index, line_index, projection_weight)
            })
            .collect();

        let mut cell_indices_to_check = Vec::new();
        let mut dominating_line_indices = Vec::new();
        let mut summed_projection_weights = Vec::new();

        for i in 0..results.len() {
            let (flat_index, line_index, projection_weight) = results[i];

            if projection_weight > model.sampling_settings.weight_limit {
                cell_indices_to_check.push(flat_index);
                dominating_line_indices.push(line_index);
                summed_projection_weights.push(projection_weight);
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
            summed_projection_weights
        }

        
    }
}
