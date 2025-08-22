// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Various corrections for the circulation strength and angle of attack calculated
//! 
//! The module contains an implementation block for the [LineForceModel] and various correction 
//! structs.
//! 
//! # Available methods for the LineForceModel
//! - [`circulation_strength_smoothed`](LineForceModel::circulation_strength_smoothed)
//! - [`gaussian_smoothed_values`](LineForceModel::gaussian_smoothed_values)
//! - [`polynomial_smoothed_values`](LineForceModel::polynomial_smoothed_values)
//! - [`circulation_strength_prescribed`](LineForceModel::circulation_strength_prescribed)

pub mod circulation;
pub mod angle_of_attack;

use stormath::smoothing::{
    gaussian::GaussianSmoothing, 
    polynomial::CubicPolynomialSmoothing
};

use circulation::prescribed::{
    PrescribedCirculation,
    PrescribedCirculationShape
};
use circulation::smoothing::{
    CirculationSmoothing,
    SmoothingType
};

use super::*;

impl LineForceModel {
    /// Returns the circulation strength with smoothing applied.
    /// 
    /// # Arguments
    /// - `velocity` - A vector of velocities at the control points.
    /// - `input_coordinate_system` - The coordinate system in which the input velocity is given.
    /// - `circulation_smoothing` - The smoothing settings to be applied to the circulation
    ///   strength.
    pub fn circulation_strength_smoothed(
        &self, 
        velocity: &[SpatialVector], 
        input_coordinate_system: CoordinateSystem,
        circulation_smoothing: &CirculationSmoothing,
    ) -> Vec<f64> {
        let raw_strength = self.circulation_strength_raw(
            velocity, 
            input_coordinate_system
        );

        let values_to_subtract = if let Some(prescribed_circulation) = &circulation_smoothing.prescribed_to_subtract_before_smoothing {
            self.circulation_strength_prescribed(
                velocity, 
                input_coordinate_system,
                prescribed_circulation
            )
        } else {
            vec![0.0; raw_strength.len()]
        };

        let values_to_smooth: Vec<f64> = raw_strength.iter()
            .zip(values_to_subtract.iter())
            .map(|(raw, subtract)| raw - subtract)
            .collect();

        let smoothed_values = match circulation_smoothing.smoothing_type {
            SmoothingType::Gaussian(ref settings) => {
                self.gaussian_smoothed_values(&values_to_smooth, settings)
            },
            SmoothingType::CubicPolynomial(ref settings) => {
                self.polynomial_smoothed_values(&values_to_smooth, settings)
            }
        };

        smoothed_values.iter()
            .zip(values_to_subtract.iter())
            .map(|(smoothed, subtract)| smoothed + subtract)
            .collect()
    }

    /// Function that applies a Gaussian smoothing to the supplied strength vector. 
    /// 
    /// # Arguments
    /// - `noisy_values` - A slice of f64 values representing the noisy values to be smoothed.
    /// - `smoothing_settings` - A slice of Gaussian smoothing settings for each wing.
    /// 
    /// # Returns
    /// A vector of smoothed values, where each value corresponds to the smoothed value of the input 
    /// noisy value.
    pub fn gaussian_smoothed_values(
        &self, 
        noisy_values: &[f64],
        smoothing_settings: &[GaussianSmoothing<f64>],
    ) -> Vec<f64> {        
        let mut smoothed_values: Vec<f64> = Vec::with_capacity(noisy_values.len());
        
        let span_distance = self.span_distance_in_local_coordinates();

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let settings = &smoothing_settings[wing_index];
            
            let local_span_distance = span_distance[wing_indices.clone()].to_vec();
            let local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let wing_smoothed_values = settings.apply_smoothing(
                &local_span_distance, 
                &local_noisy_values, 
            );
            
            for index in 0..wing_smoothed_values.len() {
                smoothed_values.push(wing_smoothed_values[index]);
            }
        }

        smoothed_values
    }

    pub fn polynomial_smoothed_values(
        &self, 
        noisy_values: &[f64],
        smoothing_settings: &[CubicPolynomialSmoothing<f64>],
    ) -> Vec<f64> {
        let mut smoothed_values: Vec<f64> = Vec::with_capacity(noisy_values.len());

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {

            let local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let settings = &smoothing_settings[wing_index];
            let wing_smoothed_values = settings.apply_smoothing(&local_noisy_values);

            for index in 0..wing_smoothed_values.len() {
                smoothed_values.push(wing_smoothed_values[index]);
            }
        }

        smoothed_values
    }

    /// Returns a circulation distribution that is forced to follow the specified distribution 
    /// shape, but where the averaged value still is the same as what is calculated from the raw 
    /// lifting line theory.
    /// 
    /// The shape is enforced on the circulation divided by the square of the velocity,
    /// so that changes due to external velocity variations (e.g., velocity due to rotational 
    /// motion, atmospheric boundary layer, interactions with a another structure, etc.) are taken 
    /// into account.
    /// 
    /// # Arguments
    /// * `velocity` - The velocity at the control points, used to calculate the circulation
    /// * `prescribed_circulation` - The shape of the circulation distribution to be followed
    /// * `input_coordinate_system` - The coordinate system in which the input velocity is given.
    pub fn circulation_strength_prescribed(
        &self, 
        velocity: &[SpatialVector], 
        input_coordinate_system: CoordinateSystem,
        prescribed_circulation: &PrescribedCirculation, 
    ) -> Vec<f64> {

        let nr_wings = self.nr_wings();

        let raw_circulation_strength = self.circulation_strength_raw(
            velocity, 
            input_coordinate_system
        );

        let velocity_squared: Vec<f64> = velocity.iter().map(
            |v| v.length_squared()
        ).collect();

        let mut gamma_divided_by_u2 = Vec::with_capacity(raw_circulation_strength.len());

        for i in 0..raw_circulation_strength.len() {
            if velocity_squared[i] < f64::MIN_POSITIVE {
                gamma_divided_by_u2.push(0.0);
            } else {
                gamma_divided_by_u2.push(raw_circulation_strength[i] / velocity_squared[i]);
            }
        }

        let averaged_gamma_divided_by_u2 = self.wing_averaged_values(
            &gamma_divided_by_u2
        );

        let effective_relative_span_distance = self.effective_relative_span_distance();

        let prescribed_shape_vector = if prescribed_circulation.curve_fit_shape_parameters {
            let mut local_shape_vector = Vec::with_capacity(nr_wings);
            
            for wing_index in 0..nr_wings {
                let wing_indices = self.wing_indices[wing_index.clone()].clone();

                local_shape_vector.push(
                    PrescribedCirculationShape::from_curve_fit(
                        &effective_relative_span_distance[wing_indices.clone()], 
                        &gamma_divided_by_u2[wing_indices.clone()], 
                        &prescribed_circulation.shape.as_params_vector()
                    )
                )
            }

            local_shape_vector
        } else {
            vec![prescribed_circulation.shape.clone(); nr_wings]
        };

        let mut out: Vec<f64> = Vec::with_capacity(raw_circulation_strength.len());

        for wing_index in 0..nr_wings {
            let wing_indices = self.wing_indices[wing_index].clone();

            let local_effective_relative_span_distance = effective_relative_span_distance[wing_indices.clone()].to_vec();
            let local_velocity_squared = velocity_squared[wing_indices.clone()].to_vec();

            let local_prescribed_circulation_shape_values = prescribed_shape_vector[wing_index].get_values(
                &local_effective_relative_span_distance
            );

            let averaged_averaged_prescribed_circulation_value: f64 = local_prescribed_circulation_shape_values.iter()
                .sum::<f64>() / (local_prescribed_circulation_shape_values.len() as f64);

            for i in 0..local_effective_relative_span_distance.len() {
                let factor = if averaged_averaged_prescribed_circulation_value == 0.0 {
                    0.0
                } else {
                    averaged_gamma_divided_by_u2[wing_index] / 
                    averaged_averaged_prescribed_circulation_value
                };

                out.push(
                    factor * 
                    local_prescribed_circulation_shape_values[i] * 
                    local_velocity_squared[i]
                );
            }
        }

        out
    }
}