// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormbird::wind::{
    environment::WindEnvironment as WindEnvironmentRust, 
    wind_condition::WindCondition
};

use stormath::spatial_vector::SpatialVector;

use pyo3::prelude::*;

use std::ops::Range;

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
        linear_velocity,
        height = 10.0
    ))]
    pub fn apparent_wind_direction_from_condition_and_linear_velocity(
        &self,
        wind_velocity: f64, 
        wind_direction_coming_from: f64,
        linear_velocity: [f64; 3],
        height: f64
    ) -> f64 {
        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction_coming_from
        };
        
        let linear_velocity_internal = SpatialVector::from(linear_velocity);
        
        self.data.apparent_wind_direction_from_condition_and_linear_velocity(
            wind_condition,
            linear_velocity_internal,
            height
        )
    }
    
    #[pyo3(signature=(
        *,
        wind_velocity,
        wind_direction_coming_from,
        ctrl_points,
        linear_velocity,
        wing_indices
    ))]
    pub fn apparent_wind_velocity_vectors_at_ctrl_points_with_corrections_applied(
        &self,
        wind_velocity: f64, 
        wind_direction_coming_from: f64,
        ctrl_points: Vec<[f64; 3]>,
        linear_velocity: [f64; 3],
        wing_indices: Vec<[usize; 2]>
    ) -> Vec<[f64; 3]> {
        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction_coming_from
        };
        
        let linear_velocity_internal = SpatialVector::from(linear_velocity);
        
        let mut wing_indices_internal: Vec<Range<usize>> = Vec::new();
        
        for i in 0..wing_indices.len() {
            wing_indices_internal.push(
                Range{
                    start: wing_indices[i][0],
                    end: wing_indices[i][1]
                }
            );
        }
        
        let mut ctrl_points_internal = Vec::new();
        
        for i in 0..ctrl_points.len() {
            ctrl_points_internal.push(SpatialVector::from(ctrl_points[i]));
        }
        
        let velocity_internal = self.data.apparent_wind_velocity_vectors_at_ctrl_points_with_corrections_applied(
            wind_condition, 
            &ctrl_points_internal, 
            linear_velocity_internal, 
            &wing_indices_internal
        );
        
        let mut velocity_out: Vec<[f64; 3]> = Vec::new();
        
        for i in 0..velocity_internal.len() {
            velocity_out.push(
                [velocity_internal[i][0], velocity_internal[i][1], velocity_internal[i][2]]
            )
        }
        
        velocity_out
    }
}