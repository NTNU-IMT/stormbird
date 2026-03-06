// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::ops::Range;

/// Functionality to represent the a wind environment

use stormath::{
    type_aliases::Float,
    spatial_vector::SpatialVector
};
use serde::{Serialize, Deserialize};
use serde_json;

use crate::error::Error;
use crate::line_force_model::LineForceModel;
use super::inflow_corrections::InflowCorrections;
use super::wind_condition::WindCondition;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Structure used to represent a wind environment. Can be used to query about the wind velocity at
/// different locations.
pub struct WindEnvironment {
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
    
    #[inline(always)]
    /// Returns the height from a given vector, based on the definitions on the up-direction and 
    /// water plane height given in the structure.
    pub fn height_from_location(&self, location: SpatialVector) -> Float {
        (
            location.dot(self.up_direction) - self.water_plane_height
        ).max(0.0)
    }
    
    #[inline(always)]
    /// Returns the true wind direction as a vector
    pub fn true_wind_direction_vector(&self, direction_coming_from: Float) -> SpatialVector {
        self.zero_direction_vector.rotate_around_axis(
            direction_coming_from,
            self.wind_rotation_axis
        )
    }
    
    pub fn steady_true_wind_velocity_at_location(
        &self,
        condition: &WindCondition,
        location: SpatialVector,
    ) -> Float {
        let height = self.height_from_location(location);

        condition.steady_true_wind_velocity_at_height(height)
    }

    /// Computes the height of the input location and then the true wind velocity at this height
    pub fn unsteady_parallel_true_wind_velocity_at_location(
        &self,
        condition: &WindCondition,
        location: SpatialVector,
        time: Float
    ) -> Float {
        let height = self.height_from_location(location);

        condition.unsteady_parallel_true_wind_velocity_at_height(height, time)
    }

    /// Returns the true wind vector at the location given as input
    pub fn unsteady_true_wind_velocity_vector_at_location(
        &self,
        condition: &WindCondition,
        location: SpatialVector,
        time: Float
    ) -> SpatialVector {
        let parallel_vel = self.unsteady_parallel_true_wind_velocity_at_location(
            condition, 
            location, 
            time
        );
        let parallel_dir = self.true_wind_direction_vector(condition.direction_coming_from);
        
        let perpendicular_vel = condition.unsteady_perpendicular_true_wind_velocity(time);
        let perpendicular_dir = self.up_direction.cross(parallel_dir);
        
        let vertical_dir = self.up_direction;
        let vertical_vel = condition.unsteady_vertical_true_wind_velocity(time);

        parallel_vel * parallel_dir + 
        perpendicular_vel * perpendicular_dir + 
        vertical_vel * vertical_dir
    }
    
    /// Returns the true wind vector at the location given as input
    pub fn steady_true_wind_velocity_vector_at_location(
        &self,
        condition: &WindCondition,
        location: SpatialVector,
    ) -> SpatialVector {
        let parallel_vel = self.steady_true_wind_velocity_at_location(condition, location);
        let parallel_dir = self.true_wind_direction_vector(condition.direction_coming_from);

        parallel_vel * parallel_dir
    }
    
    pub fn steady_apparent_wind_velocity_vector_at_location(
        &self,
        condition: &WindCondition,
        location: SpatialVector,
        linear_velocity: SpatialVector,
    ) -> SpatialVector {
        let true_wind = self.steady_true_wind_velocity_vector_at_location(
            condition, 
            location
        );
        
        true_wind + linear_velocity
    }
    
    pub fn unsteady_apparent_wind_velocity_vector_at_location(
        &self,
        condition: &WindCondition,
        location: SpatialVector,
        linear_velocity: SpatialVector,
        time: Float
    ) -> SpatialVector {
        let true_wind = self.unsteady_true_wind_velocity_vector_at_location(
            condition, 
            location, 
            time
        );
        
        true_wind + linear_velocity
    }

    pub fn apparent_wind_velocity_vectors_at_ctrl_points_with_corrections_applied(
        &self,
        condition: &WindCondition,
        ctrl_points: &[SpatialVector],
        linear_velocity: SpatialVector,
        time: Float,
        wing_indices: &[Range<usize>]
    ) -> Vec<SpatialVector> {
        
        let nr_ctrl_points = ctrl_points.len();
        
        let mut wind_velocity = Vec::with_capacity(nr_ctrl_points);
        let mut average_height = 0.0;
        
        for i in 0..nr_ctrl_points {
            wind_velocity.push(
                self.unsteady_apparent_wind_velocity_vector_at_location(
                    condition, 
                    ctrl_points[i], 
                    linear_velocity, 
                    time
                )
            );
            
            average_height += ctrl_points[i].dot(self.up_direction);
        }
        
        average_height /= nr_ctrl_points as Float;
        
        let apparent_wind_direction = self.apparent_wind_direction_from_condition_and_linear_velocity(
            condition, linear_velocity, average_height
        );
        
        self.apply_inflow_corrections(
            apparent_wind_direction,
            &mut wind_velocity,
            ctrl_points,
            wing_indices,
        );

        wind_velocity
    }

    /// Applies inflow corrections to the first points in the input freestream velocity
    pub fn apply_inflow_corrections(
        &self,
        apparent_wind_direction: Float,
        freestream_velocity: &mut [SpatialVector],
        ctrl_points: &[SpatialVector],
        wing_indices: &[Range<usize>]
    ) {
        if let Some(corrections) = &self.inflow_corrections {
            let nr_ctrl_points = ctrl_points.len();
            let nr_wings = wing_indices.len();
            
            let mut height_values: Vec<Float> = Vec::with_capacity(
                nr_ctrl_points
            );
            
            for i in 0..nr_ctrl_points {
                height_values.push(
                    ctrl_points[i].dot(self.up_direction)
                );
            }
            
            for wing_index in 0..nr_wings {
                for i in wing_indices[wing_index].start..wing_indices[wing_index].end {
                    freestream_velocity[i] = corrections.correct_velocity_single_sail(
                        wing_index,
                        apparent_wind_direction,
                        height_values[i],
                        freestream_velocity[i],
                        self.up_direction
                    )
                }
            }
        }
    }

    pub fn apparent_wind_direction_from_condition_and_linear_velocity(
        &self,
        condition: &WindCondition,
        linear_velocity: SpatialVector,
        height: Float
    ) -> Float {
        let location = height * self.up_direction;
        
        let true_wind_vector = self.steady_true_wind_velocity_vector_at_location(
            condition,
            location
        );
        
        let apparent_velocity_vector = true_wind_vector + linear_velocity;

        self.zero_direction_vector.signed_angle_between(
            apparent_velocity_vector,
            self.wind_rotation_axis
        )
    }

    /// Measures the apparent wind direction based on the input velocity vectors, where the sign and
    /// magnitude is defined by the zero_direction_vector and the wind_rotation_axis.
    pub fn apparent_wind_direction_from_velocity_based_on_rotation_axis(
        &self,
        velocity: SpatialVector
    ) -> Float {
        self.zero_direction_vector.signed_angle_between(
            velocity,
            self.wind_rotation_axis
        )
    }

    /// Measures the apparent wind direction based on the input velocity vectors, where the sign is
    /// defined by the local, non-rotated, chord vector and rotation-axis of each wing in the line
    /// force model. This, then, gives the wind direction relative to the local coordinate system
    /// for each wing. A direction of zero means that the flow is aligned with the non-rotated chord
    pub fn apparent_wind_direction_from_velocity_and_line_force_model(
        &self,
        velocity: &[SpatialVector],
        line_force_model: &LineForceModel
    ) -> Vec<Float> {

        let nr_span_lines = line_force_model.nr_span_lines();

        let mut out = Vec::with_capacity(nr_span_lines);

        for i in 0..nr_span_lines {
            let wing_index = line_force_model.wing_index_from_global(i);

            let first_strip_index = line_force_model.wing_indices[wing_index].start;

            let rotation_axis = line_force_model.span_lines_global[first_strip_index].relative_vector().normalize();

            let chord_local_non_transformed = line_force_model.chord_vectors_local_not_rotated[first_strip_index];

            let chord_local_transformed = line_force_model.rigid_body_motion.transform_vector(chord_local_non_transformed);

            out.push(
                chord_local_transformed.signed_angle_between(
                    velocity[i],
                    rotation_axis
                )
            );
        }

        out
    }
}

/* 
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
            direction_coming_from: 0.0,
            ..Default::default()
        };

        let east_wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: Float::from(90.0).to_radians(),
            ..Default::default()
        };

        let west_wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: Float::from(-90.0).to_radians(),
            ..Default::default()
        };

        let south_wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: Float::from(180.0).to_radians(),
            ..Default::default()
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

*/
