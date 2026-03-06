// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

pub mod velocity_variation;
pub mod discretized_spectrum;

use velocity_variation::VelocityVariation;
use discretized_spectrum::DiscretizedSpectrum;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindCondition {
    pub direction_coming_from: Float,
    pub velocity_variation: VelocityVariation,
    #[serde(default)]
    pub parallel_gust: Option<DiscretizedSpectrum>,
    #[serde(default)]
    pub perpendicular_gust: Option<DiscretizedSpectrum>,
    #[serde(default)]
    pub vertical_gust: Option<DiscretizedSpectrum>
}

impl WindCondition {
    /// Returns the true velocity magnitude at the height gives and input. 
    pub fn steady_true_wind_velocity_at_height(&self, height: Float) -> Float {
        self.velocity_variation.true_wind_velocity_at_height(height)
    }
    
    pub fn unsteady_parallel_true_wind_velocity_at_height(&self, height: Float, time: Float) -> Float {
        let mut u = self.velocity_variation.true_wind_velocity_at_height(height);
        
        if let Some(spectrum) = &self.parallel_gust {
            u += spectrum.value_at_time(time);
        }
        
        u
    }
    
    pub fn unsteady_perpendicular_true_wind_velocity(&self, time: Float) -> Float {
        if let Some(spectrum) = &self.perpendicular_gust {
            spectrum.value_at_time(time)
        } else {
            0.0
        }
    }
    
    pub fn unsteady_vertical_true_wind_velocity(&self, time: Float) -> Float {
        if let Some(spectrum) = &self.vertical_gust {
            spectrum.value_at_time(time)
        } else {
            0.0
        }
    }
    
    pub fn set_parallel_gust_from_json_string(&mut self, gust_string: &str) {
        let parallel_gust: DiscretizedSpectrum = serde_json::from_str(gust_string).unwrap();
        
        self.parallel_gust = Some(parallel_gust);
    }
    
    pub fn set_perpendicular_gust_from_json_string(&mut self, gust_string: &str) {
        let perpendicular_gust: DiscretizedSpectrum = serde_json::from_str(&gust_string).unwrap();
        
        self.perpendicular_gust = Some(perpendicular_gust);
    }
    
    pub fn set_vertical_gust_from_json_string(&mut self, gust_string: &str) {
        let vertical_gust: DiscretizedSpectrum = serde_json::from_str(gust_string).unwrap();
        
        self.vertical_gust = Some(vertical_gust);
    }
}

/*
impl WindCondition {
    pub fn from_velocity_vector_assuming_ned(
        velocity_vector: SpatialVector,
    ) -> Self {

        let direction_coming_from = velocity_vector.signed_angle_between(NED_ZERO_DIRECTION, NED_UP_DIRECTION);

        WindCondition {
            velocity: velocity_vector.length(),
            direction_coming_from,
            ..Default::default()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_velocity_vector() {
        let wind_velocity = 8.2;
        let allowable_error = 1e-6;

        // x-axis points forward -> negative x component is wind from the north
        let north_wind_vector = SpatialVector::new(-wind_velocity, 0.0, 0.0);
        
        // y-axis points towards the east -> negative y component is wind from the east side
        let east_wind_vector = SpatialVector::new(0.0, -wind_velocity, 0.0);

        let south_wind_vector = SpatialVector::new(wind_velocity, 0.0, 0.0);

        let west_wind_vector = SpatialVector::new(0.0, wind_velocity, 0.0);

        let north_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            north_wind_vector
        );

        let east_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            east_wind_vector
        );

        let south_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            south_wind_vector
        );

        let west_wind_condition = WindCondition::from_velocity_vector_assuming_ned(
            west_wind_vector
        );

        assert!((north_wind_condition.velocity - wind_velocity).abs() < allowable_error);
        assert!((south_wind_condition.velocity - wind_velocity).abs() < allowable_error);
        assert!((east_wind_condition.velocity - wind_velocity).abs() < allowable_error);
        assert!((west_wind_condition.velocity - wind_velocity).abs() < allowable_error);

        assert!(north_wind_condition.direction_coming_from.abs() < allowable_error);
        assert!((south_wind_condition.direction_coming_from.to_degrees().abs() - 180.0).abs() < allowable_error);
        assert!((east_wind_condition.direction_coming_from.to_degrees() - 90.0).abs() < allowable_error);
        assert!((west_wind_condition.direction_coming_from.to_degrees() + 90.0).abs() < allowable_error);
    }

}*/
