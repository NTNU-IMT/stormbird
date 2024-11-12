// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interpolation functions

use super::smoothing::gaussian_kernel;

/// Linear interpolation of a target value based on input data.
///
/// # Arguments
/// * `x_target` - The x target value to interpolate to.
/// * `x_data` - The input x data.
/// * `y_data` - The input y data.
pub fn linear_interpolation<T>(x_target: f64, x_data: &[f64], y_data: &[T]) -> T
where T:
    std::ops::Mul<f64, Output = T> +
    std::ops::Add<T, Output = T> +
    std::ops::Sub<T, Output = T> +
    Copy
{
    assert_eq!(x_data.len(), y_data.len(), "x_data and y_data must have the same length");

    match x_data.len() {
        0 => panic!("x_data is empty"),
        1 => y_data[0],
        2 => {
            let local_x_data = [x_data[0], x_data[1]];
            let local_y_data = [y_data[0], y_data[1]];

            linear_interpolation_two_data_points(x_target, &local_x_data, &local_y_data)
        },
        _ => {
            let index_min = binary_search(x_target, x_data);

            if index_min == x_data.len() - 1 {
                y_data[index_min]
            } else {
                let local_x_data = [x_data[index_min], x_data[index_min + 1]];
                let local_y_data = [y_data[index_min], y_data[index_min + 1]];

                linear_interpolation_two_data_points(x_target, &local_x_data, &local_y_data)
            }

        }
    }
}

/// Binary search for the index of the largest element in a slice that is less than or equal to
/// a target value.
///
/// # Arguments
/// * `x_target` - The target value to search for.
/// * `x_data` - The data to search in.
pub fn binary_search<T>(x_target: T, x_data: &[T]) -> usize
where T:
    PartialOrd
{
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

/// Linear interpolation of a target value based on two input data points.
///
/// # Arguments
/// * `x_target` - The x target value to interpolate to.
/// * `x_data` - The input x data.
/// * `y_data` - The input y data.
pub fn linear_interpolation_two_data_points<T>(x_target: f64, x_data: &[f64; 2], y_data: &[T; 2]) -> T
where T:
    std::ops::Mul<f64, Output = T> +
    std::ops::Add<T, Output = T> +
    std::ops::Sub<T, Output = T> +
    Copy
{
    if x_target <= x_data[0] {
        y_data[0]
    } else if x_target >= x_data[1] {
        y_data[1]
    } else {
        let delta_y = y_data[1] - y_data[0];
        let delta_x = x_data[1] - x_data[0];

        y_data[0] + delta_y * ((x_target - x_data[0]) / delta_x)
    }
}


/// Gaussian interpolation of a target value based on input data.
///
/// # Arguments
/// * `x_target` - The x target value to interpolate to.
/// * `x_data` - The input x data.
/// * `y_data` - The input y data.
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
    fn linear_interpolation_test() {
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

    #[test]
    fn binary_search_test() {
        let x_data = vec![0.0, 1.0];

        let x_target = 0.999999;

        let index = binary_search(x_target, &x_data);

        assert_eq!(index, 0);
    }
}
