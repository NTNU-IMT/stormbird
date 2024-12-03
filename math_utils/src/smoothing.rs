// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Smoothing functions for 1D data.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmoothingEndConditions {
    ZeroValues,
    MirroredValues,
    ReversedMirroredValues,
}

impl SmoothingEndConditions {
    pub fn add_end_values_to_x_data(&self, x: &[f64], number_of_end_insertions: usize) -> Vec<f64> {
        let mut x_modified: Vec<f64> = Vec::with_capacity(x.len() + number_of_end_insertions * 2);

        // Add start values
        let x_start = x[0];
        for i in 0..number_of_end_insertions {
            let delta_x = x[number_of_end_insertions - i] - x_start; // positive value

            x_modified.push(x_start - delta_x);
        }

        // Add interior values
        x_modified.extend_from_slice(x);

        // Add end values
        let last_index = x.len() - 1;
        let x_end = x[last_index];
        for i in 0..number_of_end_insertions {
            let delta_x = x_end - x[last_index - i - 1];

            x_modified.push(x_end + delta_x);
        }

        x_modified
    }

    pub fn add_end_values_to_y_data(&self, y: &[f64], number_of_end_insertions: usize) -> Vec<f64> {
        match self {
            SmoothingEndConditions::ZeroValues => {
                let mut y_modified: Vec<f64> = Vec::with_capacity(y.len() + number_of_end_insertions * 2);

                // Add start values
                for _ in 0..number_of_end_insertions {
                    y_modified.push(0.0);
                }

                // Add interior values
                y_modified.extend_from_slice(y);

                // Add end values
                for _ in 0..number_of_end_insertions {
                    y_modified.push(0.0);
                }

                y_modified
            }
            SmoothingEndConditions::MirroredValues => {
                let mut y_modified: Vec<f64> = Vec::with_capacity(y.len() + number_of_end_insertions * 2);

                // Add start values
                for i in 0..number_of_end_insertions {
                    y_modified.push(y[number_of_end_insertions - i]);
                }

                // Add interior values
                y_modified.extend_from_slice(y);

                // Add end values
                let last_index = y.len() - 1;
                for i in 0..number_of_end_insertions {
                    y_modified.push(y[last_index - i]);
                }

                y_modified
            }
            SmoothingEndConditions::ReversedMirroredValues => {
                let mut y_modified: Vec<f64> = Vec::with_capacity(y.len() + number_of_end_insertions * 2);

                // Add start values
                let last_index = y.len() - 1;
                for i in 0..number_of_end_insertions {
                    y_modified.push(y[last_index - number_of_end_insertions + i]);
                }

                // Add interior values
                y_modified.extend_from_slice(y);

                // Add end values
                for i in 0..number_of_end_insertions {
                    y_modified.push(y[number_of_end_insertions - i]);
                }

                y_modified
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaussianSmoothing {
    pub smoothing_length: f64,
    pub end_conditions: SmoothingEndConditions,
}


pub fn second_order_smoothing<T>(x: &[T], smoothing_factor: f64) -> Vec<T>
where T:
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    Copy
{
    let mut x_smooth: Vec<T> = Vec::with_capacity(x.len());

    x_smooth.push(x[0]);

    for i in 1..x.len()-1 {
        x_smooth.push(
            x[i] + (x[i-1] - x[i] * 2.0 + x[i+1]) * smoothing_factor
        );
    }

    x_smooth.push(*x.last().unwrap());

    x_smooth
}

/// Gaussian kernel to be used in smoothing function.
/// Based on: <https://en.wikipedia.org/wiki/Kernel_smoother>
pub fn gaussian_kernel(x: f64, x0: f64, smoothing_length: f64) -> f64 {
    let x_prime = x - x0;

    (-x_prime.powi(2) / (2.0 * smoothing_length.powi(2))).exp()
}

/// Gaussian smoothing using the kernel function above.
/// Based on: <https://en.wikipedia.org/wiki/Kernel_smoother>
pub fn gaussian_smoothing<T>(x: &[f64], y: &[T], smoothing_length: f64) -> Vec<T>
where T:
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::ops::Div<f64, Output = T> +
    Default +
    Copy
{
    let n = y.len();

    let mut y_smooth: Vec<T> = Vec::with_capacity(n);

    for i_0 in 0..n {
        let mut kernel_sum = 0.0;
        let mut kernel_func_product_sum: T = Default::default();

        for i in 0..n {
            let kernel_value = gaussian_kernel(x[i], x[i_0], smoothing_length);

            kernel_sum += kernel_value;
            kernel_func_product_sum = kernel_func_product_sum + y[i] * kernel_value;
        }

        y_smooth.push(kernel_func_product_sum / kernel_sum);
        
    }

    y_smooth
}

/// Based on the Savitzky-Golay filter, with a cubic polynomial and a window size of 5.
pub fn polynomial_smoothing<T>(y: &[T]) -> Vec<T>
where T:
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::ops::Div<f64, Output = T> +
    std::ops::Neg<Output = T> +
    Default +
    Copy
{
    let n = y.len();

    let weights = [-3.0, 12.0, 17.0, 12.0, -3.0];
    let normalization = 35.0;

    let window_offset = 2;

    let mut y_smooth: Vec<T> = Vec::with_capacity(n);

    for i in 0..n {
        if i < window_offset || i >= n - window_offset {
            y_smooth.push(y[i]);
            continue;
        }

        let mut y_smooth_i: T = Default::default();

        for j in 0..weights.len() {
            y_smooth_i = y_smooth_i + y[i+j-window_offset] * weights[j];
        }

        y_smooth.push(y_smooth_i / normalization);
    }

    y_smooth
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoothing_end_conditions() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 3.0, 2.0, 1.0];

        println!("ZeroValues");
        let end_conditions = SmoothingEndConditions::ZeroValues;
        let number_of_end_insertions = 2;

        let x_modified = end_conditions.add_end_values_to_x_data(&x, number_of_end_insertions);
        let y_modified = end_conditions.add_end_values_to_y_data(&y, number_of_end_insertions);

        dbg!(&x_modified);
        dbg!(&y_modified);

        let x_result = vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let y_result = vec![0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 2.0, 1.0, 0.0, 0.0];

        assert_eq!(y_modified, y_result);
        assert_eq!(x_modified, x_result);

        println!("MirroredValues");
        let end_conditions = SmoothingEndConditions::MirroredValues;
        let number_of_end_insertions = 2;

        let x_modified = end_conditions.add_end_values_to_x_data(&x, number_of_end_insertions);
    }
}