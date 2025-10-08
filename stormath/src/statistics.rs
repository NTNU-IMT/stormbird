// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functions that compute statistical properties of vectors

use crate::integration::trapz;
use crate::type_aliases::Float;

/// Computes the mean value of the input vector.
pub fn mean<T>(x: &[T]) -> T
where T: 
    std::ops::Div<Float, Output = T> + 
    std::ops::Add<T, Output = T> + 
    Copy
{
    let mut sum: T = x[0];

    for i in 1..x.len() {
        sum = sum + x[i];
    }

    sum / (x.len() as Float)
}

pub fn time_averaged_mean<T>(time: &[Float], x: &[T]) -> T
where T:
    std::ops::Mul<Float, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::default::Default +
    Copy
{
    let integrated_value = trapz(time, x);

    let average_factor = 1.0 / (time[time.len() - 1] - time[0]);

    integrated_value * average_factor
}

/// Returns the maximum value in the input vector.
pub fn max(x: &[Float]) -> Float {
    let mut max = x[0];

    for i in 1..x.len() {
        if x[i] > max {
            max = x[i];
        }
    }

    max
}

/// Returns the minimum value in the input vector.
pub fn min(x: &[Float]) -> Float {
    let mut min = x[0];

    for i in 1..x.len() {
        if x[i] < min {
            min = x[i];
        }
    }

    min
}