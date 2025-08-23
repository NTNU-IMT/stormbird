// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation block for methods that operate on generic values that is connected to the line
//! force model.
//! 
//! # Available methods
//! - [`wing_averaged_values`](LineForceModel::wing_averaged_values)
//! - [`interpolate_values_to_spanwise_location`](LineForceModel::interpolate_values_to_spanwise_location)
//! - [`section_values_from_wing_values`](LineForceModel::section_values_from_wing_values)
//! - [`span_point_values_from_ctrl_point_values`](LineForceModel::span_point_values_from_ctrl_point_values)

use super::*;

impl LineForceModel {
    /// General function for calculating wing-averaged values from a vector of sectional values.
    /// 
    /// # Arguments
    /// * `sectional_values` - A vector of values for each section of the wings
    /// 
    /// # Returns
    /// A vector of wing-averaged values, where each value corresponds to a wing in the model.
    /// The values are calculated by averaging the sectional values for each wing.
    pub fn wing_averaged_values<T>(&self, sectional_values: &[T]) -> Vec<T>
    where
        T: std::ops::Div<Float, Output = T> + std::ops::Add<T, Output = T> + Copy,
    {
        let mut result: Vec<T> = Vec::new();

        for wing_indices in &self.wing_indices {
            result.push(mean(&sectional_values[wing_indices.clone()]));
        }

        result
    }

    /// Function for interpolating vector values to a specified relative spanwise distance for each
    /// wing.
    pub fn interpolate_values_to_spanwise_location<T>(&self, spanwise_location: Float, sectional_values: &[T]) -> Vec<T>
    where T:
        std::ops::Mul<Float, Output = T> +
        std::ops::Add<T, Output = T> +
        std::ops::Sub<T, Output = T> +
        Copy
    {
        let mut result: Vec<T> = Vec::with_capacity(self.nr_wings());

        let relative_span_distance = self.relative_span_distance();

        for wing_indices in &self.wing_indices {
            result.push(
                linear_interpolation(
                    spanwise_location,
                    &relative_span_distance[wing_indices.clone()],
                    &sectional_values[wing_indices.clone()]
                )
            )
        }

        result
    }

    /// Maps a vector of values for each wing to a vector of values for each section
    pub fn section_values_from_wing_values<T>(&self, wing_values: &[T]) -> Vec<T>
    where
        T: Clone,
    {
        let mut result: Vec<T> = Vec::new();

        for (wing_nr, wing_indices) in self.wing_indices.iter().enumerate() {
            let wing_value = &wing_values[wing_nr];

            for _ in wing_indices.clone() {
                result.push(wing_value.clone());
            }
        }

        result
    }

    /// Maps the values at the control points to the values at the span points using linear
    /// interpolation.
    pub fn span_point_values_from_ctrl_point_values<T>(
        wing_indices: &[Range<usize>],
        ctrl_point_values: &[T],
        extrapolate_ends: bool,
    ) -> Vec<T>
    where
        T: std::ops::Add<T, Output = T>
            + std::ops::Sub<T, Output = T>
            + std::ops::Mul<Float, Output = T>
            + Copy,
    {   
        let nr_span_lines = ctrl_point_values.len();
        let nr_wings = wing_indices.len();

        let mut span_point_values: Vec<T> =
            Vec::with_capacity(nr_span_lines + nr_wings);

        for wing_index in 0..nr_wings {
            let first_index = wing_indices[wing_index].start;

            // First point is extrapolated
            if extrapolate_ends {
                let first_delta =
                    ctrl_point_values[first_index] - ctrl_point_values[first_index + 1];

                span_point_values.push(ctrl_point_values[first_index] + first_delta);
            } else {
                span_point_values.push(ctrl_point_values[first_index]);
            }

            // Loop over all span lines in the wing
            for i in wing_indices[wing_index].clone() {
                let last_index = wing_indices[wing_index].clone().last().unwrap();

                // Last point is extrapolated, all others are interpolated
                if i == last_index {
                    if extrapolate_ends {
                        let last_delta =
                            ctrl_point_values[last_index] - ctrl_point_values[last_index - 1];
                        span_point_values.push(ctrl_point_values[last_index] + last_delta);
                    } else {
                        span_point_values.push(ctrl_point_values[last_index]);
                    }
                } else {
                    span_point_values.push((ctrl_point_values[i] + ctrl_point_values[i + 1]) * 0.5);
                }
            }
        }

        span_point_values
    }

}