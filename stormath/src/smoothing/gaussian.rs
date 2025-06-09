// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

/// Gaussian kernel to be used in smoothing function.
/// Based on: <https://en.wikipedia.org/wiki/Kernel_smoother>
pub fn gaussian_kernel(x: f64, x0: f64, smoothing_length: f64) -> f64 {
    let x_prime = x - x0;

    (-x_prime.powi(2) / (2.0 * smoothing_length.powi(2))).exp()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GaussianSmoothing<T: SmoothingOps> {
    pub smoothing_length: f64,
    pub end_conditions: [EndCondition<T>; 2],
    #[serde(default)]
    pub number_of_end_insertions: Option<usize>,
    #[serde(default = "default_delta_x_factor_end_insertions")]
    pub delta_x_factor_end_insertions: f64,
    #[serde(default)]
    pub number_of_end_points_to_interpolate: usize
}

fn default_delta_x_factor_end_insertions() -> f64 {1.0}

impl<T: SmoothingOps> GaussianSmoothing<T> {
    /// Returns the number of end-insertions, which is either the user specified value, if present,
    /// or a default calculation based on the smoothing length
    pub fn number_of_end_insertions(&self, x: &[f64]) -> usize {
        let dx = (x[1] - x[0]) * self.delta_x_factor_end_insertions;

        match self.number_of_end_insertions {
            Option::Some(value) => value,
            Option::None => {
                let n = (4.0 * self.smoothing_length / dx).ceil() as usize;

                n.max(1).min(x.len())
            }
        }
    }

    /// Gaussian smoothing using the kernel function above.
    /// Based on: <https://en.wikipedia.org/wiki/Kernel_smoother>
    pub fn apply_smoothing(&self, x: &[f64], y: &[T]) -> Vec<T> {   
        let number_of_end_insertions = self.number_of_end_insertions(x);

        let x_modified_end_insertions = EndCondition::<f64>::add_end_values_to_x_data(
            x, 
            number_of_end_insertions,
            self.delta_x_factor_end_insertions
        );

        let y_modified_end_insertions = EndCondition::add_end_values_to_y_data(
            y, 
            number_of_end_insertions, 
            self.end_conditions
        );

        let n_mod_end_insertions = x_modified_end_insertions.len();

        let mut x_modified = Vec::with_capacity(n_mod_end_insertions);
        let mut y_modified: Vec<T> = Vec::with_capacity(n_mod_end_insertions);

        if self.number_of_end_points_to_interpolate > 0 {
            for i in 0..number_of_end_insertions {
                x_modified.push(x_modified_end_insertions[i]);
                y_modified.push(y_modified_end_insertions[i]);
            }

            let interior_start_index = number_of_end_insertions + self.number_of_end_points_to_interpolate;
            let interior_end_index = x_modified_end_insertions.len() - number_of_end_insertions - self.number_of_end_points_to_interpolate;

            for i in interior_start_index..interior_end_index {
                x_modified.push(x_modified_end_insertions[i]);
                y_modified.push(y_modified_end_insertions[i]);
            }

            let end_start_index = x_modified_end_insertions.len() - number_of_end_insertions;
            let end_end_index = x_modified_end_insertions.len();
            for i in end_start_index..end_end_index {
                x_modified.push(x_modified_end_insertions[i]);
                y_modified.push(y_modified_end_insertions[i]);
            }

        } else {
            for i in 0..n_mod_end_insertions {
                x_modified.push(x_modified_end_insertions[i]);
                y_modified.push(y_modified_end_insertions[i]);
            }
        }

        let n = y.len();
        let n_mod = y_modified.len();

        let mut y_smooth: Vec<T> = Vec::with_capacity(n);

        // i_0 represents the index of the point to be smoothed
        for i_0 in 0..n {
            let mut kernel_sum = 0.0;
            let mut kernel_func_product_sum: T = Default::default();

            for i in 0..n_mod {
                let kernel_value = gaussian_kernel(x_modified[i], x[i_0], self.smoothing_length);

                kernel_sum += kernel_value;
                kernel_func_product_sum = kernel_func_product_sum + y_modified[i] * kernel_value;
            }

            y_smooth.push(kernel_func_product_sum / kernel_sum);
        }

        y_smooth
    }
}



