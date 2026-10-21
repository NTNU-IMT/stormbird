
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Numerical solvers, such as root-finding algorithms.

use crate::type_aliases::Float;
use crate::error::Error;

#[derive(Debug, Clone)]
/// General structure for passing in settings to a numerical solver
pub struct SolverSettings {
    /// Tolerance for the solver to stop iterating
    pub tolerance: Float,
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
    f: impl Fn(Float) -> Float, 
    x1: Float, 
    x2: Float, 
    settings: &SolverSettings
) -> Float {
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

/// Brent's method for finding the root of a function within a bracket.
///
/// The method combines bisection, the secant method and inverse quadratic interpolation. The root
/// is always kept within a bracket where the function changes sign, which means that the method is
/// guaranteed to converge, also for functions with flat regions or other features that can cause 
/// the secant method to fail. At the same time, it usually converges as fast as the secant method
/// for smooth functions.
///
/// For more: <https://en.wikipedia.org/wiki/Brent%27s_method>
///
/// # Arguments
/// * `f` - The function to find the root of. Must be a function of one variable.
/// * `lower` - The lower end of the bracket.
/// * `upper` - The upper end of the bracket. The function must have a different sign at the two 
/// ends of the bracket.
/// * `settings` - The settings for the solver. The solver stops when the absolute value of the 
/// function is below the tolerance, or when the bracket can not be reduced further. If the 
/// maximum number of iterations is reached, the current best estimate is returned.
///
/// # Errors
/// Returns an error if the function does not change sign between the two ends of the bracket.
pub fn brent_solver(
    f: impl Fn(Float) -> Float,
    lower: Float,
    upper: Float,
    settings: &SolverSettings
) -> Result<Float, Error> {
    let mut a = lower;
    let mut b = upper;
    let mut fa = f(a);
    let mut fb = f(b);

    if fa == 0.0 {
        return Ok(a);
    }
    if fb == 0.0 {
        return Ok(b);
    }

    if fa.signum() == fb.signum() {
        return Err(Error::NoSolution(format!(
            "the function does not change sign in the bracket [{}, {}]: f({}) = {}, f({}) = {}",
            lower, upper, lower, fa, upper, fb
        )));
    }

    // The point c is always on the other side of the root than b, so that [b, c] is a bracket.
    let mut c = a;
    let mut fc = fa;

    // Step sizes of the current and previous iteration
    let mut d = b - a;
    let mut e = d;

    for _ in 0..settings.max_iterations {
        if fb.signum() == fc.signum() {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }

        // Make sure b is the best estimate
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }

        let x_tolerance = 2.0 * Float::EPSILON * b.abs();
        let half_bracket = 0.5 * (c - b);

        if fb.abs() < settings.tolerance || half_bracket.abs() <= x_tolerance {
            return Ok(b);
        }

        if e.abs() >= x_tolerance && fa.abs() > fb.abs() {
            // Try interpolation
            let s = fb / fa;

            let (mut p, mut q) = if a == c {
                // Secant step
                (2.0 * half_bracket * s, 1.0 - s)
            } else {
                // Inverse quadratic interpolation
                let q = fa / fc;
                let r = fb / fc;

                (
                    s * (2.0 * half_bracket * q * (q - r) - (b - a) * (r - 1.0)),
                    (q - 1.0) * (r - 1.0) * (s - 1.0)
                )
            };

            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }

            let max_accepted_step = (3.0 * half_bracket * q - (x_tolerance * q).abs())
                .min((e * q).abs());

            if 2.0 * p < max_accepted_step {
                // Accept the interpolation
                e = d;
                d = p / q;
            } else {
                // Interpolation failed, use bisection
                d = half_bracket;
                e = d;
            }
        } else {
            // The bounds are decreasing too slowly, use bisection
            d = half_bracket;
            e = d;
        }

        a = b;
        fa = fb;

        if d.abs() > x_tolerance {
            b += d;
        } else {
            b += x_tolerance * half_bracket.signum();
        }

        fb = f(b);
    }

    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_function(x: Float) -> Float {
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

    #[test]
    fn test_brent_solver() {
        let settings = SolverSettings {
            tolerance: 1e-12,
            max_iterations: 50
        };

        let x = brent_solver(test_function, 0.0, 5.0, &settings).unwrap();
        assert!((x - 2.0).abs() < 1e-10);

        // The order of the bracket should not matter
        let x = brent_solver(test_function, 5.0, 0.0, &settings).unwrap();
        assert!((x - 2.0).abs() < 1e-10);

        let x = brent_solver(|x: Float| x.cos() - x, 0.0, 1.0, &settings).unwrap();
        assert!((x.cos() - x).abs() < 1e-12);
    }

    #[test]
    fn test_brent_solver_with_flat_region() {
        // Constant for x < 1, where the secant method would fail if both initial points are there
        let f = |x: Float| (x - 1.0).max(0.0).sqrt() - 0.5;

        let settings = SolverSettings {
            tolerance: 1e-12,
            max_iterations: 100
        };

        let x = brent_solver(f, 0.0, 3.0, &settings).unwrap();
        assert!((x - 1.25).abs() < 1e-10);
    }

    #[test]
    fn test_brent_solver_without_sign_change() {
        let settings = SolverSettings {
            tolerance: 1e-12,
            max_iterations: 50
        };

        assert!(brent_solver(test_function, 3.0, 5.0, &settings).is_err());
    }
}
