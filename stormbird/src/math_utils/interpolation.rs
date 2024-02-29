// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interpolation functions

use super::smoothing::gaussian_kernel;

/// Returns a value that is linearly interpolated on the input values.
///
/// # Arguments
/// * `x_target` - the x value for the interpolated value
/// * `x_data` - x values for the input data used in the interpolation
/// * `y_dara` - y values for the input data used in the interpolation. The y values can be of any
/// type that supports the mathematical operations necessary to perform linear interpolation.
pub fn linear_interpolation<T>(x_target: f64, x_data: &[f64], y_data: &[T]) -> T 
where T: 
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    Copy
{
    match x_data.len() {
        0 => panic!("x_data is empty"),
        1 => y_data[0],
        2 => linear_interpolation_two_data_points(x_target, x_data, y_data),
        _ => {
            let index_min = binary_search(x_target, x_data);

            if index_min == x_data.len() - 1 {
                y_data[index_min]
            } else {
                linear_interpolation_two_data_points(x_target, &x_data[index_min..index_min+2], &y_data[index_min..index_min+2])
            }
            
        }
    }
}

pub fn binary_search(x_target: f64, x_data: &[f64]) -> usize {
    let mut index_min = 0;
    let mut index_max = x_data.len() - 1;

    while index_max - index_min > 1 {
        let index_mid = (index_min + index_max) / 2;

        if x_data[index_mid] > x_target {
            index_max = index_mid;
        } else {
            index_min = index_mid;
        }
    }

    index_min
}

pub fn linear_interpolation_two_data_points<T>(x_target: f64, x_data: &[f64], y_data: &[T]) -> T 
where T: 
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    Copy
{
    if x_target <= x_data[0] {
        y_data[0]
    } else if x_target >= *x_data.last().unwrap() {
        *y_data.last().unwrap()
    } else {
        let mut x_min = x_data[0];
        let mut x_max = *x_data.last().unwrap();
        let mut y_min = y_data[0];
        let mut y_max = *y_data.last().unwrap();

        for i in 0..x_data.len() {
            if x_data[i] > x_min && x_data[i] <= x_target {
                x_min = x_data[i];
                y_min = y_data[i];
            } else if x_data[i] < x_max && x_data[i] >= x_target {
                x_max = x_data[i];
                y_max = y_data[i];
            }
        }

        let delta_y = y_max - y_min;

        y_min + delta_y * ((x_target - x_min) / (x_max - x_min))
    }
}

pub fn linear_array_interpolation<T>(x_target_array: &[f64], x_data: &[f64], y_data: &[T]) -> Vec<T>
where T: 
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    Copy
{
    let mut y_target_array: Vec<T> = Vec::with_capacity(x_target_array.len());

    for x_target in x_target_array {
        y_target_array.push(linear_interpolation(*x_target, x_data, y_data));
    }

    y_target_array
}

pub fn gaussian_interpolation<T>(x_target: f64, x_data: &[f64], y_data: &[T], smoothing_length: f64) -> T
where T:
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::ops::Div<f64, Output = T> +
    Default +
    Copy
{
    let n_data = x_data.len();

    let mut kernel_sum = 0.0;
    let mut kernel_func_product_sum: T = Default::default();

    for i in 0..n_data {
        let kernel_value = gaussian_kernel(x_data[i], x_target, smoothing_length);

        kernel_sum += kernel_value;
        kernel_func_product_sum = kernel_func_product_sum + y_data[i] * kernel_value;
    }

    kernel_func_product_sum / kernel_sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_interpolation() {
        let allowable_error = 1e-6;

        let x_data = vec![0.0, 1.0, 2.0, 4.0, 5.0];

        let linear_factor = 2.0;

        let y_data: Vec<f64> = x_data.iter().map(|x| linear_factor * x).collect();

        let x_target = 3.0;

        let y_int = linear_interpolation(x_target, &x_data, &y_data);

        dbg!(y_int);

        let y_comp = linear_factor * x_target;

        let error = (y_int - y_comp).abs();

        assert!(error < allowable_error)
    }
}