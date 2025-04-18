// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to represent the a wind environment

use stormath::spatial_vector::SpatialVector;
use serde::{Serialize, Deserialize};

pub mod height_variation;

use height_variation::HeightVariationModel;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindEnvironment {
    /// A constant velocity component, independent of position. Primarily meant to represent the 
    /// velocity due to the motion of a vessel.
    pub constant_velocity: SpatialVector<3>,
    /// The reference wind velocity at the reference height. This value is used to as input when 
    /// computing how the wind velocity varies with height.
    pub reference_wind_velocity: SpatialVector<3>,
    #[serde(default)]
    pub height_variation_model: HeightVariationModel,
    #[serde(default="WindEnvironment::default_up_direction")]
    pub up_direction: SpatialVector<3>,
    #[serde(default)]
    pub water_plane_height: f64,
}

impl WindEnvironment {
    pub fn default_up_direction() -> SpatialVector<3> {SpatialVector([0.0, 0.0, 1.0])}

    pub fn velocity_at_location(&self, location: &SpatialVector<3>) -> SpatialVector<3> {
        let height = (
            location.dot(self.up_direction) - self.water_plane_height
        ).max(0.0);

        let increase_factor = if height > 0.0 {
            self.height_variation_model.velocity_increase_factor(height)
        } else {
            0.0
        };
        
        self.constant_velocity + self.reference_wind_velocity * increase_factor
    }
}
