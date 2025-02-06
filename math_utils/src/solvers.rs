
#[derive(Debug, Clone)]
pub struct SolverSettings {
    pub tolerance: f64,
    pub max_iterations: usize,
}

/// Secant method for finding the root of a function.
/// For more: <https://en.wikipedia.org/wiki/Secant_method>
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