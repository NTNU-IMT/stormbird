// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormbird::wind::{
    environment::WindEnvironment as WindEnvironmentRust, 
    wind_condition::WindCondition
};

use stormath::spatial_vector::SpatialVector;

use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct WindEnvironment {
    pub data: WindEnvironmentRust
}

#[pymethods]
impl WindEnvironment {
    #[new]
    pub fn new(
        setup_string: String
    ) -> Self {
        Self {
            data: WindEnvironmentRust::from_json_string(
                &setup_string
            ).unwrap()
        }
    }
    
    #[pyo3(signature=(
        *,
        wind_velocity,
        wind_direction_coming_from,
        height
    ))]
    pub fn true_wind_velocity_at_height(
        &self, 
        wind_velocity: f64, 
        wind_direction_coming_from: f64,
        height: f64
    ) -> f64 {
        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction_coming_from
        };
        
        self.data.true_wind_velocity_at_height(wind_condition, height)
    }
    
    #[pyo3(signature=(
        *,
        wind_velocity,
        wind_direction_coming_from,
        location
    ))]
    pub fn true_wind_velocity_vector_at_location(
        &self, 
        wind_velocity: f64, 
        wind_direction_coming_from: f64,
        location: [f64; 3]
    ) -> [f64; 3] {
        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction_coming_from
        };
        
        let location_internal = SpatialVector::from(location);
        
        self.data.true_wind_velocity_vector_at_location(wind_condition, location_internal).0
    }
    
    #[pyo3(signature=(
        *,
        wind_velocity,
        wind_direction_coming_from,
        location,
        linear_velocity
    ))]
    pub fn apparent_wind_velocity_vector_at_location(
        &self, 
        wind_velocity: f64, 
        wind_direction_coming_from: f64,
        location: [f64; 3],
        linear_velocity: [f64; 3]
    ) -> [f64; 3] {
        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction_coming_from
        };
        
        let location_internal = SpatialVector::from(location);
        let linear_velocity_internal = SpatialVector::from(linear_velocity);
        
        self.data.apparent_wind_velocity_vector_at_location(
            wind_condition, 
            location_internal,
            linear_velocity_internal
        ).0
    }
    
    #[pyo3(signature=(
        *,
        wind_velocity,
        wind_direction_coming_from,
        linear_velocity
    ))]
    pub fn apparent_wind_direction_from_condition_and_linear_velocity(
        &self,
        wind_velocity: f64, 
        wind_direction_coming_from: f64,
        linear_velocity: [f64; 3]
    ) -> f64 {
        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction_coming_from
        };
        
        let linear_velocity_internal = SpatialVector::from(linear_velocity);
        
        self.data.apparent_wind_direction_from_condition_and_linear_velocity(
            wind_condition,
            linear_velocity_internal
        )
    }
}