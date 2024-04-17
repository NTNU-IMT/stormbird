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