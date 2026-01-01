// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! *Special functions* means functions that are well known, but which otherwise are just functions
//! that produce a value based on some input. The name *special functions* is inspired by the
//! the scipy library.

use crate::type_aliases::Float;
use crate::consts::PI;

/// A sigmoid function where the value goes from zero to one.
///
/// ## Arguments
/// * `x` - The input value to the function
/// * `x0` - The x-value where the function is equal to 0.5
/// * `transition_range` - The range around x0 where the function transitions from close to 0 to 
/// close to 1. The
///
/// ## Returns
/// The value of the sigmoid function at the input
pub fn sigmoid_zero_to_one(x: Float, x0: Float, transition_range: Float) -> Float {
    // The slope constant is set such that this function returns 0.01 at x = x0 - transition_range
    // and 0.99 at x = x0 + transition_range
    let slope = 4.5951212 / transition_range;

    let x_prime = slope * (x - x0);

    1.0 / ( 1.0 + Float::exp(-x_prime))
}

/// A cosine transition from zero to one
/// 
/// ## Arguments
/// * `x` - The input value to the function
/// * `x_start` - The x-value where the function should be zero
/// * `transition_range` - The distance over which the function transitions from zero to one
/// 
/// ## Returns
/// A value between 0 and 1 using a cosine transition function
pub fn cosine_transition_zero_to_one(x: Float, x_start: Float, transition_range: Float) -> Float {
    let x_prime = x - x_start;

    if x_prime > transition_range {
        return 1.0;
    } else if x_prime < 0.0 {
        return 0.0;
    }
    
    let omega = PI / transition_range;
    
    0.5 * (1.0 - (x_prime * omega).cos())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sigmoid() {
        let x0 = 2.4;
        let transition_range = 1.2;

        let f0 = super::sigmoid_zero_to_one(x0, x0, transition_range);

        let f_n = super::sigmoid_zero_to_one(x0 - transition_range, x0, transition_range);
        let f_p = super::sigmoid_zero_to_one(x0 + transition_range, x0, transition_range);

        dbg!(f0, f_n, f_p);

        assert!((f0 - 0.5).abs() < 1e-9, "f0 = {}", f0);
        assert!((f_n - 0.01).abs() < 1e-6, "f_n = {}", f_n);
        assert!((f_p - 0.99).abs() < 1e-6, "f_p = {}", f_p);
    }
    
    #[test]
    fn test_cosine_transition() {
        let x_start = 2.4;
        let transition_range = 3.14;
        
        let f_low = super::cosine_transition_zero_to_one(-1.0, x_start, transition_range);
        let f_start = super::cosine_transition_zero_to_one(x_start, x_start, transition_range);
        let f_mid = super::cosine_transition_zero_to_one(x_start + 0.5 * transition_range, x_start, transition_range);
        let f_end = super::cosine_transition_zero_to_one(x_start + transition_range, x_start, transition_range);
        let f_large = super::cosine_transition_zero_to_one(x_start + 2.3 * transition_range, x_start, transition_range);
        
        assert!(f_low.abs() < 1e-9, "f_low = {}", f_low);
        assert!(f_start.abs() < 1e-9, "f_start = {}", f_start);
        assert!((f_mid - 0.5).abs() < 1e-9, "f_mid = {}", f_mid);
        assert!((f_end - 1.0).abs() < 1e-9, "f_end = {}", f_end);
        assert!((f_large - 1.0).abs() < 1e-9, "f_large = {}", f_end);
        
        dbg!(f_low, f_start, f_mid, f_end, f_large);
    }
}