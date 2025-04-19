// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functions that compute statistical properties of vectors

/// Computes the mean value of the input vector.
pub fn mean<T>(x: &[T]) -> T
where T: 
    std::ops::Div<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    Copy
{
    let mut sum: T = x[0];

    for i in 1..x.len() {
        sum = sum + x[i];
    }

    sum / (x.len() as f64)
}

/// Returns the maximum value in the input vector.
pub fn max(x: &[f64]) -> f64 {
    let mut max = x[0];

    for i in 1..x.len() {
        if x[i] > max {
            max = x[i];
        }
    }

    max
}

/// Returns the minimum value in the input vector.
pub fn min(x: &[f64]) -> f64 {
    let mut min = x[0];

    for i in 1..x.len() {
        if x[i] < min {
            min = x[i];
        }
    }

    min
}