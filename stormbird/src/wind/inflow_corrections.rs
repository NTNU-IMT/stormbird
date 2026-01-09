// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;
use stormath::interpolation::linear_interpolation;

use std::ops::Range;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure that stores inflow correction data for multiple sails for a single apparent wind
/// direction, as a function of a scalar height value. The correction consists if a spatial vector
/// that contain correction factors for each component of the velocity.
pub struct InflowCorrectionsSingleDirection {
    pub height_values: Vec<Float>,
    pub magnitude_corrections: Vec<Float>,
    pub angle_corrections: Vec<Float>, 
    pub wing_indices: Vec<Range<usize>>,
}

impl InflowCorrectionsSingleDirection {
    pub fn correct_velocity_single_sail(
        &self,
        wing_index: usize,
        height: Float,
        velocity: SpatialVector,
        up_direction: SpatialVector,
    ) -> SpatialVector {
        let range = self.wing_indices[wing_index].clone();
        
        let magnitude_correction = linear_interpolation(
            height, 
            &self.height_values[range.clone()], 
            &self.magnitude_corrections[range.clone()]
        );
        
        let angle_correction = linear_interpolation(
            height, 
            &self.height_values[range.clone()], 
            &self.angle_corrections[range.clone()]
        );
        
        magnitude_correction * velocity.rotate_around_axis(angle_correction, up_direction)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflowCorrections {
    pub apparent_wind_directions: Vec<Float>,
    pub corrections: Vec<InflowCorrectionsSingleDirection>,
}

impl InflowCorrections {
    pub fn correct_velocity_single_sail(
        &self,
        wing_index: usize,
        apparent_wind_direction: Float,
        height: Float,
        velocity: SpatialVector,
        up_direction: SpatialVector,
    ) -> SpatialVector {
        let mut corrected_velocities: Vec<SpatialVector> = Vec::with_capacity(
            self.apparent_wind_directions.len()
        );
        
        for i in 0..self.apparent_wind_directions.len() {
            corrected_velocities.push(
                self.corrections[i].correct_velocity_single_sail(
                    wing_index,
                    height, 
                    velocity, 
                    up_direction
                )
            );
        }
        
        linear_interpolation(
            apparent_wind_direction, 
            &self.apparent_wind_directions, 
            &corrected_velocities
        )
    }
}
