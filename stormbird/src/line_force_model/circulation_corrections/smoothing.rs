// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to be used in solvers, for instance a lifting line simulation

use serde::{Serialize, Deserialize};

use math_utils::smoothing;

use crate::line_force_model::LineForceModel;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GaussianSmoothing {
    pub length_factor: f64,
}

impl LineForceModel {
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

        let end_corrections_distance_factor = 2.0;

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let smoothing_length = settings.length_factor * wing_span_lengths[wing_index];

            let non_zero_circulation_at_ends = self.non_zero_circulation_at_ends[wing_index];   

            let mut local_span_distance = span_distance[wing_indices.clone()].to_vec();
            let mut local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let start_index = if !non_zero_circulation_at_ends[0] {
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

            let end_index = if !non_zero_circulation_at_ends[1] {
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
}