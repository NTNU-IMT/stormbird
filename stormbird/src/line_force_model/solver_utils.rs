// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to be used in solvers, for instance a lifting line simulation

use super::*;

use serde::{Serialize, Deserialize};

use crate::math_utils::interpolation;
use crate::math_utils::smoothing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceTest {
    pub minimum_sucesses: usize,
    pub allowed_error: f64,
}

impl Default for ConvergenceTest {
    fn default() -> Self {
        Self {
            minimum_sucesses: 20,
            allowed_error: 1e-6,
        }
    }
}

impl ConvergenceTest {
    pub fn build(&self) -> ConvergenceTestExecutor {
        ConvergenceTestExecutor {
            number_of_sucesses: 0,
            minimum_sucesses: self.minimum_sucesses,
            allowed_error: self.allowed_error,
        }
    }
}

pub struct ConvergenceTestExecutor {
    pub number_of_sucesses: usize,
    pub minimum_sucesses: usize,
    pub allowed_error: f64,
}

impl ConvergenceTestExecutor {
    pub fn test(&mut self, max_error: f64) -> bool {
        if max_error < self.allowed_error {
            self.number_of_sucesses += 1;
        } else {
            self.number_of_sucesses = 0;
        }

        if max_error < self.allowed_error && self.number_of_sucesses >= self.minimum_sucesses {
            true
        } else {
            false
        }
    }
}

impl LineForceModel {
    pub fn smooth_strength(&self, noisy_strength: &[f64], end_correction: (bool, bool), smoothing_length_factor: f64) -> Vec<f64> {   
        let mut smoothed_strength: Vec<f64> = Vec::with_capacity(noisy_strength.len());
        let span_distance = self.relative_span_distance();

        let smoothing_length = smoothing_length_factor * self.span_lines_local[0].length();

        for wing_indices in &self.wing_indices {
            let mut corrected_noisy_strength: Vec<f64> = if end_correction.0 || end_correction.1 {
                Vec::with_capacity(wing_indices.len() + 2)
            } else {
                Vec::with_capacity(wing_indices.len())
            };

            let mut current_wing_span_distance: Vec<f64> = span_distance[wing_indices.clone()].to_vec();

            if end_correction.0 {
                corrected_noisy_strength.push(0.0);
                current_wing_span_distance.insert(0, -0.5);
            }

            for value in &noisy_strength[wing_indices.clone()] {
                corrected_noisy_strength.push(*value);
            }

            if end_correction.1 {
                corrected_noisy_strength.push(0.0);
                current_wing_span_distance.push(0.5);
            }

            let wing_smoothed_strength = smoothing::gaussian_smoothing(
                &current_wing_span_distance, &corrected_noisy_strength, smoothing_length
            );

            let start = if end_correction.0 {
                1
            } else {
                0
            };

            let end = if end_correction.1 {
                wing_smoothed_strength.len()-1
            } else {
                wing_smoothed_strength.len()
            };

            for index in start..end {
                smoothed_strength.push(wing_smoothed_strength[index]);
            }
        }

        smoothed_strength
    }

    pub fn map_velocity_linear(&self, source_velocity: &[Vec3], target_model: &LineForceModel) -> Vec<Vec3> {
        let mut target_velocity: Vec<Vec3> = Vec::with_capacity(target_model.nr_span_lines());
    
        let source_span_distance = self.relative_span_distance();
        let target_span_distance = target_model.relative_span_distance();
    
        for (source_wing_indices, target_wing_indices) in self.wing_indices.iter().zip(&target_model.wing_indices) {
            let current_wing_source_velocity: Vec<Vec3> = source_velocity[source_wing_indices.clone()].to_vec();
            let current_wing_source_span_distance: Vec<f64> = source_span_distance[source_wing_indices.clone()].to_vec();
            
            let current_wing_target_span_distance: Vec<f64> = target_span_distance[target_wing_indices.clone()].to_vec();
    
            for index in 0..target_wing_indices.len() {
                target_velocity.push(
                    interpolation::linear_interpolation(
                        current_wing_target_span_distance[index], 
                        &current_wing_source_span_distance, 
                        &current_wing_source_velocity
                    )
                );
            }
        }
    
        target_velocity
    }

    pub fn map_velocity_gaussian(&self, source_velocity: &[Vec3], target_model: &LineForceModel, gaussian_length_factor: f64) -> Vec<Vec3> {
        let mut target_velocity: Vec<Vec3> = Vec::with_capacity(target_model.nr_span_lines());
    
        let source_span_distance = self.relative_span_distance();
        let target_span_distance = target_model.relative_span_distance();
    
        for (source_wing_indices, target_wing_indices) in self.wing_indices.iter().zip(&target_model.wing_indices) {
            let current_wing_source_velocity: Vec<Vec3> = source_velocity[source_wing_indices.clone()].to_vec();
            let current_wing_source_span_distance: Vec<f64> = source_span_distance[source_wing_indices.clone()].to_vec();

            let relative_element_length = current_wing_source_span_distance[1] - current_wing_source_span_distance[0]; // TODO: Find better way to handle varying element length

            let gaussian_length = gaussian_length_factor * relative_element_length;
            
            let current_wing_target_span_distance: Vec<f64> = target_span_distance[target_wing_indices.clone()].to_vec();
    
            for index in 0..target_wing_indices.len() {
                target_velocity.push(
                    interpolation::gaussian_interpolation(
                        current_wing_target_span_distance[index], 
                        &current_wing_source_span_distance, 
                        &current_wing_source_velocity,
                        gaussian_length
                    )
                );
            }
        }
    
        target_velocity
    }

    /// Maps the strength from the source model (self) to a target model that (can) have a different 
    /// resolution. 
    pub fn map_strength_linear(&self, self_strength: &[f64], target_model: &LineForceModel, end_correction: bool) -> Vec<f64> {
        let mut target_strength: Vec<f64> = Vec::with_capacity(target_model.nr_span_lines());
    
        let span_distance_self    = self.relative_span_distance();
        let span_distance_target = target_model.relative_span_distance();
    
        for (source_wing_indices, target_wing_indices) in self.wing_indices.iter().zip(&target_model.wing_indices) {
            let mut current_wing_source_strength: Vec<f64> = self_strength[source_wing_indices.clone()].to_vec();
            let mut current_wing_source_span_distance: Vec<f64> = span_distance_self[source_wing_indices.clone()].to_vec();
            
            if end_correction {
                current_wing_source_strength.insert(0, 0.0);
                current_wing_source_strength.push(0.0);
                
                current_wing_source_span_distance.insert(0, -0.5);
                current_wing_source_span_distance.push(0.5);
            }
            
            let current_wing_target_span_distance: Vec<f64> = span_distance_target[target_wing_indices.clone()].to_vec();
    
            for index in 0..target_wing_indices.len() {
                target_strength.push(
                    interpolation::linear_interpolation(
                        current_wing_target_span_distance[index], 
                        &current_wing_source_span_distance, 
                        &current_wing_source_strength
                    )
                );
            }
        }
    
        target_strength
    }

    /// Maps the strength from the source model (self) to a target model that (can) have a different 
    /// resolution. 
    pub fn map_strength_gaussian(&self, self_strength: &[f64], target_model: &LineForceModel, end_correction: (bool, bool), gaussian_length_factor: f64) -> Vec<f64> {
        let mut target_strength: Vec<f64> = Vec::with_capacity(target_model.nr_span_lines());
    
        let span_distance_self   = self.relative_span_distance();
        let span_distance_target = target_model.relative_span_distance();

        for (source_wing_indices, target_wing_indices) in self.wing_indices.iter().zip(&target_model.wing_indices) {
            let mut current_wing_source_strength: Vec<f64> = self_strength[source_wing_indices.clone()].to_vec();
            let mut current_wing_source_span_distance: Vec<f64> = span_distance_self[source_wing_indices.clone()].to_vec();

            let relative_element_length = current_wing_source_span_distance[1] - current_wing_source_span_distance[0]; // TODO: Find better way to handle varying element length

            let gaussian_length = gaussian_length_factor * relative_element_length;
            
            if end_correction.0 {
                current_wing_source_strength.insert(0, 0.0);
                current_wing_source_span_distance.insert(0, -0.5 - gaussian_length);
            }

            if end_correction.1 {
                current_wing_source_strength.push(0.0);
                current_wing_source_span_distance.push(0.5 + gaussian_length);
            }
            
            let current_wing_target_span_distance: Vec<f64> = span_distance_target[target_wing_indices.clone()].to_vec();
    
            for index in 0..target_wing_indices.len() {
                target_strength.push(
                    interpolation::gaussian_interpolation(
                        current_wing_target_span_distance[index], 
                        &current_wing_source_span_distance, 
                        &current_wing_source_strength,
                        gaussian_length
                    )
                );
            }
        }
    
        target_strength
    }
}

