// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

/// Gaussian kernel to be used in smoothing function.
/// Based on: <https://en.wikipedia.org/wiki/Kernel_smoother>
pub fn gaussian_kernel(x: Float, x0: Float, smoothing_length: Float) -> Float {
    let x_prime = x - x0;

    (-x_prime.powi(2) / (2.0 * smoothing_length.powi(2))).exp()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GaussianSmoothing<T: SmoothingOps> {
    /// The length to use in the Gaussian kernel
    pub smoothing_length: Float,
    /// How to treat the ends
    pub end_conditions: [EndCondition<T>; 2],
    #[serde(default)]
    pub number_of_end_insertions: Option<usize>,
    #[serde(default = "default_delta_x_factor_end_insertions")]
    pub delta_x_factor_end_insertions: Float,
}

fn default_delta_x_factor_end_insertions() -> Float {1.0}

impl<T: SmoothingOps> GaussianSmoothing<T> {
    /// Returns the number of end-insertions, which is either the user specified value, if present,
    /// or a default calculation based on the smoothing length
    pub fn number_of_end_insertions(&self, x: &[Float]) -> usize {
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
    pub fn apply_smoothing(&self, x: &[Float], y: &[T]) -> Vec<T> {   
        let number_of_end_insertions = self.number_of_end_insertions(x);

        let x_modified = EndCondition::<Float>::add_end_values_to_x_data(
            x, 
            number_of_end_insertions,
            self.delta_x_factor_end_insertions
        );

        let y_modified = EndCondition::add_end_values_to_y_data(
            y, 
            number_of_end_insertions, 
            self.end_conditions
        );

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
    
    /// Apply smoothing, but with weighted smoothing length
    pub fn apply_smoothing_with_varying_smoothing_weight(
        &self,
        x: &[Float],
        y: &[T],
        smoothing_weight: &[Float]
    ) -> Vec<T> {
        let number_of_end_insertions = self.number_of_end_insertions(x);

        let x_modified = EndCondition::<Float>::add_end_values_to_x_data(
            x, 
            number_of_end_insertions,
            self.delta_x_factor_end_insertions
        );

        let y_modified = EndCondition::add_end_values_to_y_data(
            y, 
            number_of_end_insertions, 
            self.end_conditions
        );

        let n = y.len();
        let n_mod = y_modified.len();

        let mut y_smooth: Vec<T> = Vec::with_capacity(n);

        // i_0 represents the index of the point to be smoothed
        for i_0 in 0..n {
            let mut kernel_sum = 0.0;
            let mut kernel_func_product_sum: T = Default::default();
            
            let smoothing_length = self.smoothing_length * smoothing_weight[i_0];
            
            if smoothing_length.abs() < f64::MIN_POSITIVE {
                y_smooth.push(y[i_0])
            } else {
                for i in 0..n_mod {
                    let kernel_value = gaussian_kernel(x_modified[i], x[i_0], smoothing_length);
    
                    kernel_sum += kernel_value;
                    kernel_func_product_sum = kernel_func_product_sum + y_modified[i] * kernel_value;
                }
    
                y_smooth.push(kernel_func_product_sum / kernel_sum);
            }
        }

        y_smooth
    }
}



