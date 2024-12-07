// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to be used in solvers, for instance a lifting line simulation

use serde::{Serialize, Deserialize};

use math_utils::smoothing;
use math_utils::smoothing::end_condition::EndCondition;

use crate::line_force_model::LineForceModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaussianSmoothing {
    #[serde(default="GaussianSmoothing::default_length_factor")]
    /// A non dimensional factor used to calculate the length in Gaussian smoothing kernel. 
    /// The actual smoothing length is calculated as the length factor times the wing span.
    pub length_factor: f64
}

impl GaussianSmoothing {
    fn default_length_factor() -> f64 {0.1}
}

impl Default for GaussianSmoothing {
    fn default() -> Self {
        Self {
            length_factor: Self::default_length_factor(),
        }
    }
}

impl LineForceModel {
    pub fn smoothing_end_conditions(&self, wing_index: usize) -> [EndCondition; 2] {
        let non_zero_circulation_at_ends = self.non_zero_circulation_at_ends[wing_index];

        let first_end_condition = if non_zero_circulation_at_ends[0] {
            EndCondition::ExtendedValues
        } else {
            EndCondition::ZeroValues
        };

        let second_end_condition = if non_zero_circulation_at_ends[1] {
            EndCondition::ExtendedValues
        } else {
            EndCondition::ZeroValues
        };

        [first_end_condition, second_end_condition]
    }
    /// Function that applies a Gaussian smoothing to the supplied strength vector.
    pub fn gaussian_smoothed_values(
        &self, 
        noisy_values: &[f64],
        settings: &GaussianSmoothing,
    ) -> Vec<f64> {
        if settings.length_factor <= 0.0 {
            return noisy_values.to_vec();
        }

        let mut smoothed_values: Vec<f64> = Vec::with_capacity(noisy_values.len());

        let wing_span_lengths = self.wing_span_lengths();
        
        let span_distance = self.span_distance_in_local_coordinates();

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let smoothing_length = settings.length_factor * wing_span_lengths[wing_index];

            let end_conditions = self.smoothing_end_conditions(wing_index);

            let local_span_distance = span_distance[wing_indices.clone()].to_vec();
            let local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let gaussian_smoothing = smoothing::gaussian::GaussianSmoothing {
                smoothing_length,
                end_conditions,
                number_of_end_insertions: None
            };

            let raw_wing_smoothed_values = gaussian_smoothing.apply_smoothing(
                &local_span_distance, 
                &local_noisy_values, 
            );
            
            for index in 0..raw_wing_smoothed_values.len() {
                smoothed_values.push(raw_wing_smoothed_values[index]);
            }
        }

        smoothed_values
    }

    pub fn polynomial_smoothed_values(&self, noisy_values: &[f64]) -> Vec<f64> {
        let mut smoothed_values: Vec<f64> = Vec::with_capacity(noisy_values.len());

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let end_conditions = self.smoothing_end_conditions(wing_index);

            let local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let polynomial_smoothing = smoothing::polynomial::CubicPolynomialSmoothing {
                window_size: smoothing::polynomial::WindowSize::Seven,
                end_conditions: end_conditions
            };

            let wing_smoothed_values = polynomial_smoothing.apply_smoothing(&local_noisy_values);

            for index in 0..wing_smoothed_values.len() {
                smoothed_values.push(wing_smoothed_values[index]);
            }
        }

        smoothed_values
    }
}