
//! *Special functions* means functions that are well known, but which otherwise are just functions
//! that produce a value based on some input. The name *special functions* is inspired by the
//! the scipy library.

use crate::type_aliases::Float;

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
}