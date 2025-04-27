// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Numerical integration functions

/// Trapezoidal integration of the input arrays.
/// For more: <https://en.wikipedia.org/wiki/Trapezoidal_rule>
///
/// # Arguments
/// * `x` - the input values of the function to be integrated
/// * `y` - the return values of the function to be integrated. in other words, y = f(x)
pub fn trapz<T>(x: &[f64], y: &[T]) -> T 
where T:
    std::ops::Mul<f64, Output = T> + 
    std::ops::Add<T, Output = T> + 
    std::ops::Sub<T, Output = T> + 
    std::default::Default +
    Copy
{
    let mut integration_sum: T = T::default();

    for i in 1..x.len() {
        let delta_x: f64 = x[i] - x[i-1];

        let y_average: T = (y[i-1] + y[i]) * 0.5;
        
        integration_sum = integration_sum + y_average * delta_x;
    }

    integration_sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array_generation;

    #[test]
    fn polynomial_integration() {
        let allowable_error = 0.01 / 100.0;
        
        let x_min = 0.433;
        let x_max = 5.2342;

        let n = 100;
        let x_vector = array_generation::linspace(x_min, x_max, n);

        let a0 = 0.343;
        let a1 = 1.2425;
        let a2 = 4.1231;

        let mut y_vector: Vec<f64> = Vec::new();
        for x in &x_vector {
            y_vector.push(a0 + a1 * x + a2 * x.powf(2.0));
        }

        let analytical_value = a0 * (x_max - x_min) + (a1 / 2.0) * (x_max.powf(2.0) - x_min.powf(2.0)) + (a2 / 3.0) * (x_max.powf(3.0) - x_min.powf(3.0));

        let trapz_value = trapz(&x_vector, &y_vector);

        let error = ((trapz_value - analytical_value) / analytical_value).abs();

        println!("Analytical value:  {}", analytical_value);
        println!("Trapezoidal value: {}", trapz_value);
        println!("Error in trapz:    {} %", 100.0 * error);

        assert!(error < allowable_error)
    }
}