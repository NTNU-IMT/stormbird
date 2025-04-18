// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functions to perform finite difference calculations

/// Calculate the first order derivative on the data using first order finite difference scheme
/// 
/// # Arguments
/// * `data` - The data to calculate the derivative on. The last element in the array is the newest
/// data point, while the first is the oldest data point.
/// * `step_size` - The step size between the data points
pub fn first_derivative_first_order<T>(data: &[T; 2], step_size: f64) -> T 
where T: 
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::ops::Div<f64, Output = T> +
    Copy
{
    (data[1] - data[0]) / step_size
}

/// Calculates the first order derivative based on the data using second order backwards finite 
/// difference scheme
/// 
/// # Arguments
/// * `data` - The data to calculate the derivative on. The last element in the array is the newest
/// data point, while the first is the oldest data point. Each point is assumed to be equally spaced
/// based on the step size
pub fn first_derivative_second_order_backward<T>(data: &[T; 3], step_size: f64) -> T 
where T: 
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::ops::Div<f64, Output = T> +
    Copy
{
    (data[2] * 3.0 - data[1] * 4.0 + data[0]) / (2.0 * step_size)
}

/// Calculates the double derivative based on the data using a backwards finite difference scheme.
/// 
/// Source: https://en.wikipedia.org/wiki/Finite_difference
/// 
/// # Arguments
/// * `data` - The data to calculate the derivative on. The last element in the array is the newest
/// data point, while the first is the oldest data point. Each point is assumed to be equally spaced
/// based on the step size
pub fn second_derivative_backward<T>(data: &[T; 3], step_size: f64) -> T 
where T: 
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::ops::Div<f64, Output = T> +
    Copy
{
    (data[2] - data[1] * 2.0 + data[0]) / step_size.powi(2)
}

/// Calculates the spatial derivative of the data using central finite difference scheme on the 
/// interior points, and forward and backward finite difference on the end points.
pub fn derivative_spatial_arrays<T>(x_data: &[f64], y_data: &[T]) -> Vec<T> 
where T: 
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::ops::Div<f64, Output = T> +
    std::default::Default +
    Copy
{   
    assert!(x_data.len() == y_data.len(), "The x and y data must have the same length");

    let mut result = Vec::with_capacity(x_data.len());

    for i in 0..x_data.len() {
        let (delta_x, delta_y) = if i == 0 {
            (
                x_data[1] - x_data[0], 
                y_data[1] - y_data[0]
            )
        } else if i == x_data.len() - 1 {
            (
                x_data[i] - x_data[i - 1], 
                y_data[i] - y_data[i - 1]
            )
        } else {
            (
                x_data[i + 1] - x_data[i - 1], 
                y_data[i + 1] - y_data[i - 1]
            )
        };

        if delta_x == 0.0 {
            result.push(T::default());
        } else {
            result.push(delta_y / delta_x);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(x: f64) -> f64 {
        3.2 * x.powi(3) + 2.0 * x + 1.0
    }

    fn derivative(x: f64) -> f64 {
        3.0 * 3.2 * x.powi(2) + 2.0
    }

    fn second_derivative(x: f64) -> f64 {
        3.0 * 2.0 * 3.2 * x
    }

    #[test]
    fn test_derivatives() {
        let step_size = 0.0012;
        let x0 = 0.85;

        let previous_x_values: [f64; 3] = [x0, x0 + step_size, x0 + 2.0 * step_size];
        let new_x_value = x0 + 3.0 * step_size;
        
        let data: [f64; 3] = [
            function(previous_x_values[0]),
            function(previous_x_values[1]),
            function(previous_x_values[2]),
        ];

        let derivative_truth = derivative(new_x_value);
        let second_derivative_truth = second_derivative(new_x_value);

        let derivative_first_order = first_derivative_first_order(&[data[1], data[2]], step_size);
        let derivative_second_order = first_derivative_second_order_backward(&data, step_size);
        let second_derivative = second_derivative_backward(&data, step_size);

        dbg!(derivative_truth, derivative_first_order, derivative_second_order);
        dbg!(second_derivative_truth, second_derivative);

        let derivative_error_first_order = (derivative_truth - derivative_first_order).abs() / derivative_truth.abs();
        let derivative_error_second_order = (derivative_truth - derivative_second_order).abs() / derivative_truth.abs();
        let second_derivative_error = (second_derivative_truth - second_derivative).abs() / second_derivative_truth.abs();
        
        dbg!(derivative_error_first_order, derivative_error_second_order, second_derivative_error);

        let allowed_error = 0.005;
        assert!(derivative_error_first_order < allowed_error);
        assert!(derivative_error_second_order < allowed_error);
        assert!(second_derivative_error < allowed_error);
    }
}