

use crate::type_aliases::Float;
use crate::matrix::Matrix;

use super::bounded_variable::BoundedVariable;

#[derive(Debug, Clone)]
/// Settings for fitting data to a function using non-linear least squares. The implementation 
/// follows the [Levenberg-Marquard method](https://en.wikipedia.org/wiki/Levenberg%E2%80%93Marquardt_algorithm)
pub struct CurveFit {
    pub function: fn(Float, &[Float]) -> Float,
    pub max_iterations: usize,
    pub delta_params: Float,
    pub initial_damping_factor: Float,
    pub damping_change_factor: Float,
    pub param_bounds: Option<Vec<BoundedVariable>>,
    pub tolerance: Float
}

impl Default for CurveFit {
    fn default() -> Self {
        Self {
            function: |_, _| 0.0,
            max_iterations: 1000,
            delta_params: 0.0001,
            initial_damping_factor: 1.0,
            damping_change_factor: 10.0,
            param_bounds: None,
            tolerance: 1e-12
        }
    }
}

impl CurveFit {
    /// Transform parameters from unbounded to bounded space
    fn transform_params_to_bounded(&self, unbounded_params: &[Float]) -> Vec<Float> {
        if let Some(bounds) = &self.param_bounds {
            unbounded_params.iter().enumerate().map(|(i, &param)| {
                if i < bounds.len() {
                    bounds[i].transform_to_bounded(param)
                } else {
                    param
                }
            }).collect()
        } else {
            unbounded_params.to_vec()
        }
    }

    /// Transform parameters from bounded to unbounded space
    fn transform_params_to_unbounded(&self, bounded_params: &[Float]) -> Vec<Float> {
        if let Some(bounds) = &self.param_bounds {
            bounded_params.iter().enumerate().map(|(i, &param)| {
                if i < bounds.len() {
                    bounds[i].transform_to_unbounded(param)
                } else {
                    param
                }
            }).collect()
        } else {
            bounded_params.to_vec()
        }
    }


    /// Calculates the Jacobian matrix for the given function and data points.
    /// # Arguments:
    /// - `function`: A function that takes a single `Float` and a slice of `Float` parameters, returning a
    ///   `Float` value representing the output of the function.
    /// - `x_data`: a slice of `Float` representing the independent variable data points.
    pub fn jacobian_matrix(
        &self,
        x_data: &[Float], 
        unbounded_params: &[Float]
    ) -> Matrix<Float> {
        let nr_data_points = x_data.len();
        let nr_params = unbounded_params.len();
        
        let mut jacobian_matrix = Matrix::new_default([nr_data_points, nr_params]);

        for i in 0..nr_data_points {
            let x_i = x_data[i];

            for j in 0..nr_params {
                let mut unbounded_params_plus = unbounded_params.to_vec();
                let mut unbounded_params_minus = unbounded_params.to_vec();

                unbounded_params_plus[j] += self.delta_params;
                unbounded_params_minus[j] -= self.delta_params;

                let bounded_plus = self.transform_params_to_bounded(&unbounded_params_plus);
                let bounded_minus = self.transform_params_to_bounded(&unbounded_params_minus);

                let f_plus = (self.function)(x_i, &bounded_plus);
                let f_minus = (self.function)(x_i, &bounded_minus);

                jacobian_matrix[[i, j]] = (f_plus - f_minus) / (2.0 * self.delta_params);
            }
        }

        jacobian_matrix
    }

    /// Calculates the residual vector for the given function and data points.
    /// # Arguments:
    /// - `function`: A function that takes a single `Float` and a slice of `Float` parameters, returning a
    ///   `Float` value representing the output of the function.
    /// - `x_data`: a slice of `Float` representing the independent variable data points.
    /// - `y_data`: a slice of `Float` representing the dependent variable data points.
    /// - `params`: a slice of `Float` representing the parameters of the function.
    /// # Returns:
    /// A vector of residuals, where each element is the difference between the observed value and 
    /// the predicted value from the function. The length of the vector is equal to the number of 
    /// data points.
    pub fn residual_vector(
        &self, 
        x_data: &[Float], 
        y_data: &[Float], 
        unbounded_params: &[Float]
    ) -> Vec<Float> {
        let nr_data_points = x_data.len();
        let mut residuals = vec![0.0; nr_data_points];

        let bounded_params = self.transform_params_to_bounded(unbounded_params);

        for i in 0..nr_data_points {
            let x_i = x_data[i];
            let y_i = y_data[i];
            let f_i = (self.function)(x_i, &bounded_params);
            
            residuals[i] = y_i - f_i;
        }

        residuals
    }

    pub fn cost_function(
        &self, 
        x_data: &[Float], 
        y_data: &[Float], 
        unbounded_params: &[Float]
    ) -> Float {
        let residuals = self.residual_vector(x_data, y_data, unbounded_params);
        
        residuals.iter().map(|&r| r * r).sum()
    }

    /// Run the Levenberg-Marquardt algorithm to fit a function to data points.
    /// # Arguments:
    /// - `function`: A function that takes a single `Float` and a slice of `Float` parameters, 
    ///    returning a `Float` value representing the output of the function.
    /// - `x_data`: a slice of `Float` representing the independent variable data points.
    /// - `y_data`: a slice of `Float` representing the dependent variable data points.
    /// - `initial_params`: a slice of `Float` representing the initial guess for the parameters of the
    ///   function.
    pub fn fit_parameters(
        &self,
        x_data: &[Float],
        y_data: &[Float],
        initial_params: &[Float],
    ) -> Vec<Float> {
        let nr_params = initial_params.len();
        
        let mut current_unbounded_params = self.transform_params_to_unbounded(initial_params);

        let mut damping_factor = self.initial_damping_factor;
        let mut current_cost = self.cost_function(x_data, y_data, &current_unbounded_params);

        let line_search_step_length_factors = [0.1, 0.25, 0.5, 1.0];

        for _ in 0..self.max_iterations {
            let jacobian = self.jacobian_matrix(x_data, &current_unbounded_params);

            let jacobian_transpose = jacobian.transpose();

            let residual = self.residual_vector(x_data, y_data, &current_unbounded_params);

            let matrix_org = jacobian_transpose.matrix_multiply(&jacobian);

            let diagonal = matrix_org.diagonal();

            let matrix = matrix_org + damping_factor * diagonal;

            let right_hand_side = jacobian_transpose.vector_multiply(&residual);

            let change_in_params = matrix.solve_gaussian_elimination(&right_hand_side);

            match change_in_params {
                Ok(change) => {
                    let mut best_cost = Float::INFINITY;
                    let mut best_params = current_unbounded_params.clone();

                    for factor in &line_search_step_length_factors {
                        let mut new_unbounded_params = current_unbounded_params.clone();
                        
                        for j in 0..nr_params {
                            new_unbounded_params[j] += factor * change[j];
                        }

                        let new_cost = self.cost_function(x_data, y_data, &new_unbounded_params);

                        if new_cost < best_cost {
                            best_cost = new_cost;
                            best_params = new_unbounded_params;
                        }
                    }

                    if best_cost < current_cost {
                        current_unbounded_params = best_params;
                        current_cost = best_cost;
                        damping_factor /= self.damping_change_factor;
                    } else {
                        damping_factor *= self.damping_change_factor;
                    }
                },
                Err(_) => {
                    damping_factor *= self.damping_change_factor;
                }
            }

            if current_cost < self.tolerance {
                break;
            }
        }

        self.transform_params_to_bounded(&current_unbounded_params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::array_generation::linspace;

    fn test_function_elliptical(x: Float, params: &[Float]) -> Float {
        let scale_factor = params[0];
        let inner_power = params[1];
        let outer_power = params[2];

        let base = 1.0 - (2.0 * x.abs()).powf(inner_power);

        if base <= 0.0 {
            return 0.0;
        }
        
        scale_factor * base.powf(outer_power)
    }
    
    fn test_function_poly(x: Float, params: &[Float]) -> Float {
        params[0]  + params[1] * x + params[2] * x.powi(2)
    }

    #[test]
    fn test_polynomial_curve_fit() {
        let curve_fitter = CurveFit {
            function: test_function_poly,
            param_bounds: Some(vec![
                BoundedVariable { min: -10.0, max: 10.0 },
                BoundedVariable { min: -10.0, max: 10.0 },
                BoundedVariable { min: -10.0, max: 10.0 },
            ]),
            ..Default::default()
        };

        let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let params_true = vec![1.0, 2.0, 3.0];
        let initial_params = vec![0.0, 0.0, 0.0];

        let y_data: Vec<Float> = x_data.iter()
            .map(|&x| test_function_poly(x, &params_true))
            .collect();

        let fitted_params = curve_fitter.fit_parameters(
            &x_data, 
            &y_data, 
            &initial_params
        );

        for i in 0..fitted_params.len() {
            assert!((fitted_params[i] - params_true[i]).abs() < 1e-5, 
                "Mismatch at index {}: {} != {}", 
                i, fitted_params[i], params_true[i]);
        }

        dbg!(&fitted_params);
    }

    #[test]
    fn test_elliptical_curve_fit() {
        let curve_fitter = CurveFit {
            function: test_function_elliptical,
            ..Default::default()
        };        

        let params_true = vec![2.1, 2.5, 0.7];
        let initial_params = vec![1.0, 2.0, 0.5];

        let x_data = linspace(-0.45, 0.45, 100);

        let y_data: Vec<Float> = x_data.iter()
            .map(|&x| test_function_elliptical(x, &params_true))
            .collect();

        let fitted_params = curve_fitter.fit_parameters(
            &x_data, 
            &y_data, 
            &initial_params
        );

        dbg!(&fitted_params);

        for i in 0..fitted_params.len() {
            assert!((fitted_params[i] - params_true[i]).abs() < 1e-5, 
                "Mismatch at index {}: {} != {}", 
                i, fitted_params[i], params_true[i]
            );
        }
    }
}