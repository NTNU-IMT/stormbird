// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to be used in solvers, for instance a lifting line simulation

use super::*;

use serde::{Serialize, Deserialize};

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
    /// Function that applies a Gaussian smoothing to the strength of the line force model.
    pub fn gaussian_smoothed_strength(
        &self, 
        noisy_strength: &[f64],
        smoothing_length_factor: f64,
    ) -> Vec<f64> {   
        let mut smoothed_strength: Vec<f64> = Vec::with_capacity(noisy_strength.len());

        let wing_span_lengths = self.wing_span_lengths();
        
        let span_distance = self.span_distance_in_local_coordinates();

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let smoothing_length = smoothing_length_factor * wing_span_lengths[wing_index];

            let wing_smoothed_strength = smoothing::gaussian_smoothing(
                &span_distance[wing_indices.clone()], 
                &noisy_strength[wing_indices.clone()], 
                smoothing_length
            );

            for index in 0..wing_smoothed_strength.len() {
                smoothed_strength.push(wing_smoothed_strength[index]);
            }
        }

        smoothed_strength
    }
}