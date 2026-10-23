// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality to empirically correct an input velocity to account for disturbances in the flow,
//! such as disturbances due to the deck or superstructure on a ship. 
//! 
//! The empirical model consists of two correction variables: a factor that can be used to adjust the 
//! effective velocity at different line segments, and an angle that can be used to rotate the 
//! effective velocity. The factors themselves must come from some external source. Typically, they 
//! are either set directly from an analysis of the velocity field over a ship without any sails
//! present, or tuned based on force measurements of sails on a real ship.

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;
use stormath::interpolation::linear_interpolation;

use std::ops::Range;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Structure that stores inflow correction data for multiple sails for a single apparent wind
/// direction, as a function of a scalar height value. The correction consists of both magnitude
/// corrections and angle corrections.
pub struct InflowCorrectionsSingleDirection {
    /// The height values where the corrections are given
    pub height_values: Vec<Float>,
    /// Factors multiplied with the velocity magnitude at each height
    pub magnitude_corrections: Vec<Float>,
    /// Angles, in radians, that the velocity is rotated around the up direction at each height
    pub angle_corrections: Vec<Float>, 
    /// The indices of the correction values that belong to each sail
    pub wing_indices: Vec<Range<usize>>,
}

impl InflowCorrectionsSingleDirection {
    /// Returns the corrected velocity for a single sail at the given height
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
/// Inflow corrections
pub struct InflowCorrections {
    /// The apparent wind directions where corrections are given
    pub apparent_wind_directions: Vec<Float>,
    /// The corrections for each apparent wind direction
    pub corrections: Vec<InflowCorrectionsSingleDirection>,
}

impl InflowCorrections {
    /// Returns the corrected velocity for a single sail at the given height, interpolated between the
    /// corrections for the closest apparent wind directions
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
