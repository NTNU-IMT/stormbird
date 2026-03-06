// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormbird::wind::{
    environment::WindEnvironment as WindEnvironmentRust,
};

use stormath::spatial_vector::SpatialVector;

use pyo3::prelude::*;

use std::ops::Range;

use super::wind_condition::WindCondition;

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
        condition,
        location,
    ))]
    pub fn steady_true_wind_velocity_at_location(
        &self, 
        condition: WindCondition,
        location: [f64; 3]
    ) -> f64 {
        let location_internal = SpatialVector::from(location);
        
        self.data.steady_true_wind_velocity_at_location(&condition.data, location_internal)
    }
    
    #[pyo3(signature=(
        *,
        condition,
        location,
    ))]
    pub fn steady_true_wind_velocity_vector_at_location(
        &self, 
        condition: WindCondition,
        location: [f64; 3]
    ) -> [f64; 3] {
        let location_internal = SpatialVector::from(location);
        
        self.data.steady_true_wind_velocity_vector_at_location(&condition.data, location_internal).0
    }
    
    #[pyo3(signature=(
        *,
        condition,
        location,
        time
    ))]
    pub fn unsteady_true_wind_velocity_vector_at_location(
        &self, 
        condition: WindCondition,
        location: [f64; 3],
        time: f64
    ) -> [f64; 3] {
        let location_internal = SpatialVector::from(location);
        
        self.data.unsteady_true_wind_velocity_vector_at_location(&condition.data, location_internal, time).0
    }
    
    #[pyo3(signature=(
        *,
        condition,
        location,
        linear_velocity
    ))]
    pub fn steady_apparent_wind_velocity_vector_at_location(
        &self, 
        condition: WindCondition,
        location: [f64; 3],
        linear_velocity: [f64; 3]
    ) -> [f64; 3] {        
        let location_internal = SpatialVector::from(location);
        let linear_velocity_internal = SpatialVector::from(linear_velocity);
        
        self.data.steady_apparent_wind_velocity_vector_at_location(
            &condition.data, 
            location_internal,
            linear_velocity_internal
        ).0
    }
    
    #[pyo3(signature=(
        *,
        condition,
        location,
        linear_velocity,
        time
    ))]
    pub fn unsteady_apparent_wind_velocity_vector_at_location(
        &self, 
        condition: WindCondition,
        location: [f64; 3],
        linear_velocity: [f64; 3],
        time: f64
    ) -> [f64; 3] {        
        let location_internal = SpatialVector::from(location);
        let linear_velocity_internal = SpatialVector::from(linear_velocity);
        
        self.data.unsteady_apparent_wind_velocity_vector_at_location(
            &condition.data, 
            location_internal,
            linear_velocity_internal,
            time
        ).0
    }
    
    #[pyo3(signature=(
        *,
        condition,
        linear_velocity,
        height = 10.0
    ))]
    pub fn apparent_wind_direction_from_condition_and_linear_velocity(
        &self,
        condition: WindCondition,
        linear_velocity: [f64; 3],
        height: f64
    ) -> f64 {
        
        let linear_velocity_internal = SpatialVector::from(linear_velocity);
        
        self.data.apparent_wind_direction_from_condition_and_linear_velocity(
            &condition.data,
            linear_velocity_internal,
            height
        )
    }
    
    #[pyo3(signature=(
        *,
        condition,
        ctrl_points,
        linear_velocity,
        time,
        wing_indices
    ))]
    pub fn apparent_wind_velocity_vectors_at_ctrl_points_with_corrections_applied(
        &self,
        condition: WindCondition,
        ctrl_points: Vec<[f64; 3]>,
        linear_velocity: [f64; 3],
        time: f64,
        wing_indices: Vec<[usize; 2]>
    ) -> Vec<[f64; 3]> {        
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
            &condition.data, 
            &ctrl_points_internal, 
            linear_velocity_internal,
            time,
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
