
use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormbird::line_force_model::builder::LineForceModelBuilder;
use stormbird::error::Error;

use super::Grid;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Builder for the grid, where the main job is to convert the correct resolution on the interior 
/// grid based on "cells_per_representative_length". The purpose of this is to be able to change the
/// domain size independently of the cell length.
pub struct GridBuilder {
    /// The start point of the grid that builder will construct
    pub start_point: SpatialVector,
    /// The end point of the grid that the builder will construct
    pub end_point: SpatialVector,
    #[serde(default="GridBuilder::default_cells_per_representative_length")]
    /// Specification of the cell length, but measured as number of cells per some representative
    /// length. Typically, the representative length will be the chord length of the wings present
    /// in the simulations
    pub cells_per_representative_length: [usize; 3],
    #[serde(default)]
    /// An optional "manually specified" representative length. This value is required if the 
    /// builder is to be used without any actuator line present, as some length value is always 
    /// required to calculate the size of the interior grid. However, it can also be used in cases
    /// where actuator lines are present, to manually override which length value is used.
    pub representative_length: Option<Float>,
    #[serde(default="GridBuilder::default_max_cells_per_axis_after_coarsening")]
    /// The max number of cells per axis after the mesh is coarsened as much as possible
    pub max_cells_per_axis_after_coarsening: usize
}

impl GridBuilder {
    pub fn default_cells_per_representative_length() -> [usize; 3] {[8, 8, 8]}
    pub fn default_max_cells_per_axis_after_coarsening() -> usize {7}
    
    /// The "core builder method" that constructs a Grid based on the supplied length. The typical 
    /// use case is to call this method from another builder method, which different logic to 
    /// supply the length value. See [`Self::build_from_line_force_model`] or 
    /// [`Self::build_from_internal_length`] for examples.
    pub fn build_from_specified_length(&self, length: Float) -> Grid {
        let interior_shape: [usize; 3] = std::array::from_fn(|axis_index| {
            let domain_length = self.end_point[axis_index] - self.start_point[axis_index];

            let cell_length = length / self.cells_per_representative_length[axis_index] as Float;

            let mut local_shape = (domain_length / cell_length).ceil() as usize;

            // Check that the local shape is such that it is possible to coarsen the mesh in the 
            // geometric multigrid method for the pressure step. A finer mesh, but that can be 
            // coarsened more, is generally beneficial for the computational speed, as the pressure
            // projection steps is time consuming, and the speed is very much dependent on coarsest 
            // grid.
            while Self::min_number_after_iterative_division_by_two(local_shape) > 
                self.max_cells_per_axis_after_coarsening {
                    local_shape += 1;
            }

            local_shape
        });

        Grid::new(
            self.start_point,
            self.end_point,
            interior_shape
        )
    }

    /// Builds a grid with the specified size and resolution using a line force model builder to 
    /// compute the representative length. 
    pub fn build_from_line_force_model_builder(
        &self, 
        line_force_model_builder: &LineForceModelBuilder
    ) -> Grid {
        if let Some(length) = self.representative_length {
            self.build_from_specified_length(length)
        } else {
            let mut average_chord_length = 0.0;
            let mut average_counter: usize = 0;

            for wing_builder in &line_force_model_builder.wing_builders {
                for chord_vector in &wing_builder.chord_vectors {
                    average_chord_length += chord_vector.length();
                    average_counter += 1;
                }
            }

            average_chord_length /= average_counter as Float;

            self.build_from_specified_length(average_chord_length)
        }
    }

    pub fn build_from_internal_length(&self) -> Result<Grid, Error> {
        if let Some(length) = self.representative_length {
            Result::Ok(
                self.build_from_specified_length(length)
            )
        } else {
            Result::Err(
                Error::CustomStringError(
                    "The internal representative_length variable must be specified in the \
                     GridBuilder to use the build_from_internal_length method.".to_owned()
                )
            )
        }
    }

    /// Function that iteratively divides a number by two, until it reaches its first non-even 
    /// number
    fn min_number_after_iterative_division_by_two(number: usize) -> usize {
        let mut out = number;

        while out.is_multiple_of(2) {
            out /= 2;
        }

        out
    }
}