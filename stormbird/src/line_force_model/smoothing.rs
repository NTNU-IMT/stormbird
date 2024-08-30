// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to be used in solvers, for instance a lifting line simulation

use super::*;
use serde::{Serialize, Deserialize};

use math_utils::smoothing;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmoothingSettings {
    #[serde(default)]
    pub gaussian: Option<GaussianSmoothingSettings>,
    #[serde(default)]
    pub artificial_viscosity: Option<ArtificialViscositySettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaussianSmoothingSettings {
    pub length_factor: f64,
    pub end_corrections: Vec<(bool, bool)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtificialViscositySettings {
    pub viscosity: f64,
    pub solver_iterations: usize,
    pub solver_damping: f64,
}

impl LineForceModel {
    pub fn smoothed_strength(&self, input_strength: &[f64], velocity: &[SpatialVector<3>]) -> Vec<f64> {
        let mut strength = input_strength.to_vec();

        if let Some(settings) = &self.smoothing_settings {
            if let Some(gaussian) = &settings.gaussian {
                if gaussian.length_factor > 0.0 {
                    strength = self.gaussian_smoothed_values(&strength, gaussian);
                }
            }

            if let Some(artificial_viscosity) = &settings.artificial_viscosity {
                strength = self.circulation_strength_with_viscosity(
                    &strength,
                    velocity,
                    artificial_viscosity
                );
            }
        }

        strength
    }

    /// Function that applies a Gaussian smoothing to the supplied strength vector.
    pub fn gaussian_smoothed_values(
        &self, 
        noisy_values: &[f64],
        settings: &GaussianSmoothingSettings,
    ) -> Vec<f64> {
        assert_eq!(settings.end_corrections.len(), self.nr_wings());

        let mut smoothed_values: Vec<f64> = Vec::with_capacity(noisy_values.len());

        let wing_span_lengths = self.wing_span_lengths();
        
        let span_distance = self.span_distance_in_local_coordinates();

        let end_corrections_distance_factor = 2.0;

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let smoothing_length = settings.length_factor * wing_span_lengths[wing_index];

            let end_corrections = settings.end_corrections[wing_index];

            let mut local_span_distance = span_distance[wing_indices.clone()].to_vec();
            let mut local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let start_index = if end_corrections.0 {
                let delta_span = local_span_distance[1] - local_span_distance[0];
                let mut span_to_be_inserted = 0.5 * delta_span;

                let mut number_of_insertions = 0;

                while span_to_be_inserted <= end_corrections_distance_factor * smoothing_length {
                    local_span_distance.insert(0, local_span_distance[0] - span_to_be_inserted);
                    local_noisy_values.insert(0, 0.0);
                    
                    span_to_be_inserted += if number_of_insertions == 0 {
                        0.5 * delta_span
                    } else {
                        delta_span
                    };

                    number_of_insertions += 1;
                }
                
                number_of_insertions
            } else {
                0
            };

            let end_index = if end_corrections.1 {
                let delta_span = local_span_distance[local_span_distance.len()-1] - local_span_distance[local_span_distance.len()-2];
                let mut span_to_be_inserted = 0.5 * delta_span;

                let mut number_of_insertions = 0;

                while span_to_be_inserted <= end_corrections_distance_factor * smoothing_length {
                    local_span_distance.push(local_span_distance[local_span_distance.len()-1] + span_to_be_inserted);
                    local_noisy_values.push(0.0);
                    
                    span_to_be_inserted += if number_of_insertions == 0 {
                        0.5 * delta_span
                    } else {
                        delta_span
                    };

                    number_of_insertions += 1;
                }

                local_span_distance.len() - number_of_insertions
            } else {
                local_span_distance.len()
            };

            let raw_wing_smoothed_values = smoothing::gaussian_smoothing(
                &local_span_distance, 
                &local_noisy_values, 
                smoothing_length
            );
            
            for index in start_index..end_index {
                smoothed_values.push(raw_wing_smoothed_values[index]);
            }
        }

        smoothed_values
    }

    pub fn circulation_strength_with_viscosity(
        &self, 
        input_strength: &[f64],
        velocity: &[SpatialVector<3>],
        settings: &ArtificialViscositySettings
    ) -> Vec<f64> {
        let mut new_estimated_strength = input_strength.to_vec();

        for _ in 0..settings.solver_iterations {
            let circulation_strength_second_derivative = self.circulation_strength_second_derivative(
                &new_estimated_strength
            );

            for i in 0..new_estimated_strength.len() {
                let scaling_factor = 0.5 * self.chord_vectors_local[i].length() * velocity[i].length();
                
                let viscosity_term = settings.viscosity * scaling_factor * circulation_strength_second_derivative[i];
                
                new_estimated_strength[i] = input_strength[i] + settings.solver_damping * viscosity_term;
            }
        }

        new_estimated_strength
    }

    /// Calculates the second derivative of the circulation strength on each line element. This can 
    /// be used to calculate artificial viscosity damping when estimating the circulation strength
    pub fn circulation_strength_second_derivative(&self, circulation_strength: &[f64]) -> Vec<f64> {
        let span_distance = self.span_distance_in_local_coordinates();

        let mut first_derivative = Vec::with_capacity(circulation_strength.len());

        for wing_index in 0..self.wing_indices.len() {
            let local_span_distance = &span_distance[self.wing_indices[wing_index].clone()];
            let local_circulation_strength = &circulation_strength[self.wing_indices[wing_index].clone()];

            let local_circulation_derivative = finite_difference::derivative_spatial_arrays(
                local_span_distance, 
                local_circulation_strength
            );

            first_derivative.extend(local_circulation_derivative);
        }

        let mut second_derivative = Vec::with_capacity(circulation_strength.len());

        for wing_index in 0..self.wing_indices.len() {
            let local_span_distance = &span_distance[self.wing_indices[wing_index].clone()];
            let local_circulation_derivative = &first_derivative[self.wing_indices[wing_index].clone()];

            let local_circulation_derivative = finite_difference::derivative_spatial_arrays(
                local_span_distance, 
                local_circulation_derivative
            );

            second_derivative.extend(local_circulation_derivative);
        }

        second_derivative
    }

}