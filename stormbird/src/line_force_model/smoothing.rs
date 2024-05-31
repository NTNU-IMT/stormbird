// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to be used in solvers, for instance a lifting line simulation

use super::*;
use serde::{Serialize, Deserialize};

use crate::math_utils::smoothing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmoothingSettings {
    pub gaussian_length_factor: Option<f64>,
    pub circulation_viscosity: Option<f64>,
    pub circulation_viscosity_iterations: Option<usize>,
}

impl LineForceModel {
    pub fn smoothed_strength(&self, input_strength: &[f64]) -> Vec<f64> {
        let mut strength = input_strength.to_vec();

        if let Some(settings) = &self.smoothing_settings {
            if let Some(gaussian_length_factor) = settings.gaussian_length_factor {
                strength = self.gaussian_smoothed_strength(&strength, gaussian_length_factor);
            }

            if let Some(circulation_viscosity) = settings.circulation_viscosity {
                let iterations = settings.circulation_viscosity_iterations.unwrap_or(1);

                for _ in 0..iterations {
                    strength = self.circulation_strength_with_viscosity(&strength, circulation_viscosity);
                }
            }
        }

        strength
    }

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

    pub fn circulation_strength_with_viscosity(
        &self, 
        input_strength: &[f64], 
        circulation_viscosity: f64
    ) -> Vec<f64> {
        let circulation_strength_second_derivative = self.circulation_strength_second_derivative(
            &input_strength
        );

        let mut new_estimated_strength = input_strength.to_vec();

        for i in 0..new_estimated_strength.len() {
            let viscosity_term = circulation_viscosity * circulation_strength_second_derivative[i];
            
            new_estimated_strength[i] += viscosity_term;
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