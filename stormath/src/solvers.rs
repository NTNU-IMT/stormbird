
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Numerical solvers, such as root-finding algorithms.

#[derive(Debug, Clone)]
/// General structure for passing in settings to a numerical solver
pub struct SolverSettings {
    /// Tolerance for the solver to stop iterating
    pub tolerance: f64,
    /// Maximum number of iterations to perform for the solver
    pub max_iterations: usize,
}

/// Secant method for finding the root of a function.
///
/// For more: <https://en.wikipedia.org/wiki/Secant_method>
///
/// # Arguments
/// * `f` - The function to find the root of. Must be a function of one variable.
/// * `x1` - The first initial guess for the root.
/// * `x2` - The second initial guess for the root.
/// * `settings` - The settings for the solver. See `SolverSettings` for more information.
pub fn secant_solver(
    f: impl Fn(f64) -> f64, 
    x1: f64, 
    x2: f64, 
    settings: &SolverSettings
) -> f64 {
    let f1 = f(x1);
    let f2 = f(x2);

    let (mut x_previous, mut x_previous_previous) = if f1.abs() < f2.abs() {
        (x1, x2)
    } else {
        (x2, x1)
    };

    for _ in 0..settings.max_iterations {
        let f_previous = f(x_previous);
        let f_previous_previous = f(x_previous_previous);

        let delta_x = x_previous - x_previous_previous;
        let delat_f = f_previous - f_previous_previous;

        let x_new = x_previous - f_previous * delta_x / delat_f;

        x_previous_previous = x_previous;
        x_previous = x_new;

        if f_previous.abs() < settings.tolerance {
            break;
        }
    }

    x_previous
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_function(x: f64) -> f64 {
        x.powf(2.0) - 4.0
    }

    #[test]
    fn test_secant_solver() {
        let x_sol = 2.0;

        let settings = SolverSettings {
            tolerance: 1e-6,
            max_iterations: 10
        };

        let x1 = secant_solver(test_function, -0.1, 0.2, &settings);
        let x2 = secant_solver(test_function, 0.2, -0.1, &settings);

        dbg!(x1);
        dbg!(x2);

        assert!((x1 - x_sol).abs() < 0.0001);
        assert!((x2 - x_sol).abs() < 0.0001);
    }
}