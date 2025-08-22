// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to represent the a wind environment

use stormath::spatial_vector::SpatialVector;
use serde::{Serialize, Deserialize};
use serde_json;

use crate::error::Error;

use super::height_variation::HeightVariationModel;

#[derive(Debug, Clone, Copy)]
pub struct WindCondition {
    pub reference_velocity: f64,
    pub direction_coming_from: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Structure used to represent a wind environment. Can be used to query about the wind velocity at 
/// different locations
pub struct WindEnvironment {
    #[serde(default)]
    pub height_variation_model: Option<HeightVariationModel>,
    #[serde(default="WindEnvironment::default_up_direction")]
    pub up_direction: SpatialVector<3>,
    #[serde(default="WindEnvironment::default_zero_direction_vector")]
    pub zero_direction_vector: SpatialVector<3>,
    #[serde(default)]
    pub water_plane_height: f64,
}

impl Default for WindEnvironment {
    fn default() -> Self {
        Self {
            height_variation_model: None,
            up_direction: Self::default_up_direction(),
            zero_direction_vector: Self::default_zero_direction_vector(),
            water_plane_height: 0.0
        }
    }
}

impl WindEnvironment {
    pub fn default_zero_direction_vector() -> SpatialVector<3> {SpatialVector([-1.0, 0.0, 0.0])}
    pub fn default_up_direction() -> SpatialVector<3> {SpatialVector([0.0, 0.0, 1.0])}

    pub fn from_json_string(json_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(json_string)?;
        
        Ok(serde_res)
    }

    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;
        
        Self::from_json_string(&json_string)
    }

    /// Computes the true wind velocity magnitude based on the input height
    pub fn true_wind_velocity_at_height(&self, condition: WindCondition, height: f64) -> f64 {
        let increase_factor = if let Some(model) = self.height_variation_model {
            if height > 0.0 {
                model.velocity_increase_factor(height)
            } else {
                0.0
            }
        } else {
            1.0
        };

        increase_factor * condition.reference_velocity
    }

    /// Computes the height of the input location and then the true wind velocity at this height
    pub fn true_wind_velocity_at_location(
        &self,
        condition: WindCondition,
        location: SpatialVector<3>,
    ) -> f64 {
        let height = (
            location.dot(self.up_direction) - self.water_plane_height
        ).max(0.0);

        self.true_wind_velocity_at_height(condition, height)
    }

    /// Returns the true wind vector at the location given as input
    pub fn true_wind_velocity_vector_at_location(
        &self,
        condition: WindCondition,
        location: SpatialVector<3>
    ) -> SpatialVector<3> {
        let velocity = self.true_wind_velocity_at_location(condition, location);

        let direction_vector = self.zero_direction_vector.rotate_around_axis(
            condition.direction_coming_from,
            self.up_direction
        );

        velocity * direction_vector
    }

    pub fn true_wind_velocity_vectors_at_locations(
        &self,
        condition: WindCondition,
        locations: &[SpatialVector<3>]
    ) -> Vec<SpatialVector<3>> {
        locations.iter()
            .map(
                |&location| self.true_wind_velocity_vector_at_location(condition, location)
            )
            .collect()
    }

    pub fn apparent_wind_velocity_vectors_at_locations(
        &self,
        condition: WindCondition,
        locations: &[SpatialVector<3>],
        linear_velocity: SpatialVector<3>
    ) -> Vec<SpatialVector<3>> {
        todo!()
    }
}

    
