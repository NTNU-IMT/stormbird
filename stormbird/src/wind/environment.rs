// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to represent the a wind environment

use std::ops::Range;

use stormath::{
    type_aliases::Float,
    spatial_vector::SpatialVector
};
use serde::{Serialize, Deserialize};
use serde_json;

use crate::error::Error;

use super::height_variation::HeightVariationModel;
use super::inflow_corrections::InflowCorrections;
use super::wind_condition::WindCondition;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Structure used to represent a wind environment. Can be used to query about the wind velocity at 
/// different locations
pub struct WindEnvironment {
    #[serde(default)]
    pub height_variation_model: Option<HeightVariationModel>,
    #[serde(default="WindEnvironment::default_up_direction")]
    pub up_direction: SpatialVector,
    #[serde(default="WindEnvironment::default_wind_rotation_axis")]
    pub wind_rotation_axis: SpatialVector,
    #[serde(default="WindEnvironment::default_zero_direction_vector")]
    pub zero_direction_vector: SpatialVector,
    #[serde(default)]
    pub water_plane_height: Float,
    #[serde(default)]
    pub inflow_corrections: Option<InflowCorrections>,
}

impl Default for WindEnvironment {
    fn default() -> Self {
        Self {
            height_variation_model: None,
            up_direction: Self::default_up_direction(),
            wind_rotation_axis: Self::default_wind_rotation_axis(),
            zero_direction_vector: Self::default_zero_direction_vector(),
            water_plane_height: 0.0,
            inflow_corrections: None
        }
    }
}

impl WindEnvironment {
    pub fn default_zero_direction_vector() -> SpatialVector {SpatialVector::from([1.0, 0.0, 0.0])}
    pub fn default_up_direction() -> SpatialVector {SpatialVector::from([0.0, 0.0, 1.0])}
    pub fn default_wind_rotation_axis() -> SpatialVector {SpatialVector::from([0.0, 0.0, -1.0])}

    pub fn from_json_string(json_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(json_string)?;
        
        Ok(serde_res)
    }

    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;
        
        Self::from_json_string(&json_string)
    }

    /// Computes the true wind velocity magnitude based on the input height
    pub fn true_wind_velocity_at_height(&self, condition: WindCondition, height: Float) -> Float {
        let increase_factor = if let Some(model) = self.height_variation_model {
            if height > 0.0 {
                model.velocity_increase_factor(height)
            } else {
                0.0
            }
        } else {
            1.0
        };

        increase_factor * condition.velocity
    }

    /// Computes the height of the input location and then the true wind velocity at this height
    pub fn true_wind_velocity_at_location(
        &self,
        condition: WindCondition,
        location: SpatialVector,
    ) -> Float {
        let height = (
            location.dot(self.up_direction) - self.water_plane_height
        ).max(0.0);

        self.true_wind_velocity_at_height(condition, height)
    }

    /// Returns the true wind vector at the location given as input
    pub fn true_wind_velocity_vector_at_location(
        &self,
        condition: WindCondition,
        location: SpatialVector
    ) -> SpatialVector {
        let velocity = self.true_wind_velocity_at_location(condition, location);

        let direction_vector = self.zero_direction_vector.rotate_around_axis(
            condition.direction_coming_from,
            self.wind_rotation_axis
        );

        velocity * direction_vector
    }

    pub fn true_wind_velocity_vectors_at_locations(
        &self,
        condition: WindCondition,
        locations: &[SpatialVector]
    ) -> Vec<SpatialVector> {
        locations.iter()
            .map(
                |&location| self.true_wind_velocity_vector_at_location(condition, location)
            )
            .collect()
    }

    pub fn apparent_wind_velocity_vectors_at_locations(
        &self,
        condition: WindCondition,
        locations: &[SpatialVector],
        linear_velocity: SpatialVector
    ) -> Vec<SpatialVector> {
        let mut true_wind = self.true_wind_velocity_vectors_at_locations(
            condition, 
            locations
        );

        for i in 0..true_wind.len() {
            true_wind[i] += linear_velocity;
        }

        true_wind
    }

    /// Applies inflow corrections to the first points in the input freestream velocity
    pub fn apply_inflow_corrections(
        &self,
        freestream_velocity: &mut [SpatialVector],
        non_dimensional_span_distances: &[Float],
        wing_indices_vector: Vec<Range<usize>>,
    ) {
        if let Some(corrections) = &self.inflow_corrections {
            for wing_index in 0..wing_indices_vector.len() {
                let wing_indices = wing_indices_vector[wing_index].clone();

                for i in wing_indices.start..wing_indices.end {
                    freestream_velocity[i] = corrections.correct_velocity(
                        wing_index, 
                        non_dimensional_span_distances[i], 
                        freestream_velocity[i], 
                        self.up_direction
                    )
                }
            }
        }
    }

    pub fn apparent_wind_directions_from_velocity(&self, velocity: &[SpatialVector]) -> Vec<Float> {
        velocity.iter().map(|velocity| {
            self.zero_direction_vector.signed_angle_between(
                *velocity, 
                self.wind_rotation_axis
            )
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_wind_velocity_vectors() {
        let wind_environment = WindEnvironment::default();
        let location = SpatialVector::new(0.0, 0.0, 10.0);

        let wind_velocity = 8.2;

        let north_wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: 0.0
        };

        let east_wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: Float::from(90.0).to_radians()
        };

        let west_wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: Float::from(-90.0).to_radians()
        };

        let south_wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: Float::from(180.0).to_radians()
        };

        let north_vector = wind_environment.true_wind_velocity_vector_at_location(
            north_wind_condition, location
        );

        let east_vector = wind_environment.true_wind_velocity_vector_at_location(
            east_wind_condition, location
        );

        let west_vector = wind_environment.true_wind_velocity_vector_at_location(
            west_wind_condition, location
        );

        let south_vector = wind_environment.true_wind_velocity_vector_at_location(
            south_wind_condition, location
        );

        assert!(north_vector[0] > 0.0);
        assert!(east_vector[1] < 0.0);
        assert!(west_vector[1] > 0.0);
        assert!(south_vector[0] < 0.0);

        dbg!(north_vector);
        dbg!(east_vector);
        dbg!(west_vector);
        dbg!(south_vector);
    }
}

    
