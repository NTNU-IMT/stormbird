
use stormath::type_aliases::Float;

use stormath::spatial_vector::SpatialVector;

use stormbird::line_force_model::LineForceModel;

use serde::{Serialize, Deserialize};

pub mod smoothing_field;

use smoothing_field::SmoothingField;

use super::Grid;

/// Repeatedly halves `x` while it is even, i.e. returns its odd part. This is the interior cell
/// count the geometric multigrid hierarchy bottoms out at along one axis, since
/// `Grid::multigrid_hierarchy` stops coarsening as soon as any axis has an odd number of cells.
fn smallest_number_not_divisible_by_two(x: usize) -> usize {
    if x == 0 {
        return 0;
    }

    let mut smallest_number = x;

    while smallest_number.is_multiple_of(2) {
        smallest_number /= 2;
    }

    smallest_number
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure used to build a grid. 
pub struct GridBuilder {
    pub start_point: SpatialVector,
    pub end_point: SpatialVector,
    pub cells_per_length_background: [usize; 3],
    #[serde(default="GridBuilder::default_cells_per_chord_at_wing_locations")]
    pub cells_per_chord_at_wing_locations: [usize; 3],
    #[serde(default="GridBuilder::default_relative_smoothing_length")]
    pub relative_smoothing_length: SpatialVector, 
    #[serde(default)]
    pub representative_length: Option<Float>
}

impl GridBuilder {
    pub fn default_cells_per_chord_at_wing_locations() -> [usize; 3] {[16, 16, 16]}
    pub fn default_relative_smoothing_length() -> SpatialVector {SpatialVector([2.0, 2.0, 2.0])}
    
    pub fn build_constant_from_background(&self) -> Grid {
        if let Some(representative_length) = self.representative_length {
            let interior_shape: [usize; 3] = std::array::from_fn( |axis| {
                let cell_length = representative_length / 
                    self.cells_per_length_background[axis] as Float;

                ((self.end_point[axis] - self.start_point[axis]) / cell_length).ceil() as usize
            });
    
            Grid::new(
                self.start_point,
                self.end_point,
                interior_shape
            )
        } else {
            panic!("A representative length must be given when building a mesh without actuator lines present")
        }
    }
    
    pub fn build_from_line_force_model(&self, line_force_model: &LineForceModel) -> Grid {
        let representative_length = if let Some(length) = self.representative_length {
            length
        } else {
            line_force_model.chord_lengths
                .iter()
                .sum::<Float>() / 
                (line_force_model.chord_lengths.len() as Float)
        };
        
        let smoothing_length = representative_length * self.relative_smoothing_length;

        let interior_points: [Vec<Float>; 3] = std::array::from_fn(
            |axis| {
                let background_cell_length = representative_length / 
                    self.cells_per_length_background[axis] as Float;
                
                let wing_cell_length = background_cell_length.min(
                    representative_length / 
                    self.cells_per_chord_at_wing_locations[axis] as Float
                );

                let active_points: Vec<Float> = line_force_model.span_lines_global
                    .iter()
                    .map(|line| line.ctrl_point()[axis])
                    .collect();
                
                let mut out: Vec<Float> = Vec::new();

                let cell_length_field = SmoothingField {
                    passive_value: background_cell_length,
                    active_value: wing_cell_length,
                    active_points,
                    gaussian_width: smoothing_length[axis],
                    softmax_strength: 10.0 // TODO: figure out how this should change with the chord length
                };

                let start_value = self.start_point[axis];
                let end_value = self.end_point[axis];

                let mut current_value = start_value;
                
                while current_value < end_value {
                    let cell_length = cell_length_field.eval(current_value);

                    out.push(current_value);

                    current_value += cell_length;
                }

                // `out` holds the cell vertices, so the interior cell count is `out.len() - 1`.
                // Keep appending cells until that count coarsens down to a small enough level for
                // the coarsest multigrid grid to be solved directly with a dense matrix.
                while smallest_number_not_divisible_by_two(out.len() - 1) > 7 {
                    let cell_length = cell_length_field.eval(current_value);

                    out.push(current_value);

                    current_value += cell_length;
                }

                out
            }
        );

        Grid::new_from_points(interior_points)
    }
}

