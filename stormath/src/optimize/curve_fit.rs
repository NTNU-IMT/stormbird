
use crate::matrix::Matrix;

#[derive(Debug, Clone)]
/// Settings for fitting data to a function using non-linear least squares. The implementation 
/// follows the [Levenberg-Marquard method](https://en.wikipedia.org/wiki/Levenberg%E2%80%93Marquardt_algorithm)
pub struct CurveFit {
    pub function: fn(f64, &[f64]) -> f64,
    pub max_iterations: usize,
    pub delta_params: f64,
}

impl CurveFit {
    /// Calculates the Jacobian matrix for the given function and data points.
    /// # Arguments:
    /// - `function`: A function that takes a single `f64` and a slice of `f64` parameters, returning a
    ///   `f64` value representing the output of the function.
    /// - `x_data`: a slice of `f64` representing the independent variable data points.
    pub fn jacobian_matrix(
        &self,
        x_data: &[f64], 
        params: &[f64]
    ) -> Matrix<f64> {
        let nr_data_points = x_data.len();
        let nr_params = params.len();
        
        let mut jacobian_matrix = Matrix::new_default([nr_data_points, nr_params]);

        for i in 0..nr_data_points {
            let x_i = x_data[i];

            for j in 0..nr_params {
                let mut params_plus = params.to_vec();
                let mut params_minus = params.to_vec();

                params_plus[j] += self.delta_params;
                params_minus[j] -= self.delta_params;

                let f_plus = (self.function)(x_i, &params_plus);
                let f_minus = (self.function)(x_i, &params_minus);

                jacobian_matrix[[i, j]] = (f_plus - f_minus) / (2.0 * self.delta_params);
            }
        }

        jacobian_matrix
    }

    /// Calculates the residual vector for the given function and data points.
    /// # Arguments:
    /// - `function`: A function that takes a single `f64` and a slice of `f64` parameters, returning a
    ///   `f64` value representing the output of the function.
    /// - `x_data`: a slice of `f64` representing the independent variable data points.
    /// - `y_data`: a slice of `f64` representing the dependent variable data points.
    /// - `params`: a slice of `f64` representing the parameters of the function.
    /// # Returns:
    /// A vector of residuals, where each element is the difference between the observed value and 
    /// the predicted value from the function. The length of the vector is equal to the number of 
    /// data points.
    pub fn residual_vector(
        &self, 
        x_data: &[f64], 
        y_data: &[f64], 
        params: &[f64]
    ) -> Vec<f64> {
        let nr_data_points = x_data.len();
        let mut residuals = vec![0.0; nr_data_points];

        for i in 0..nr_data_points {
            let x_i = x_data[i];
            let y_i = y_data[i];
            let f_i = (self.function)(x_i, params);
            
            residuals[i] = y_i - f_i;
        }

        residuals
    }

    /// Run the Levenberg-Marquardt algorithm to fit a function to data points.
    /// # Arguments:
    /// - `function`: A function that takes a single `f64` and a slice of `f64` parameters, 
    ///    returning a `f64` value representing the output of the function.
    /// - `x_data`: a slice of `f64` representing the independent variable data points.
    /// - `y_data`: a slice of `f64` representing the dependent variable data points.
    /// - `initial_params`: a slice of `f64` representing the initial guess for the parameters of the
    ///   function.
    pub fn fit_parameters(
        &self,
        x_data: &[f64],
        y_data: &[f64],
        initial_params: &[f64],
    ) -> Vec<f64> {
        let nr_params = initial_params.len();
        
        let mut current_params = initial_params.to_vec();

        for _ in 0..self.max_iterations {
            let jacobian = self.jacobian_matrix(x_data, &current_params);

            let jacobian_transpose = jacobian.transpose();

            let residual = self.residual_vector(x_data, y_data, &current_params);

            let matrix = jacobian_transpose.matrix_multiply(&jacobian);

            let right_hand_side = jacobian_transpose.vector_multiply(&residual);

            let change_in_params = matrix.solve_gaussian_elimination(&right_hand_side);

            for j in 0..nr_params {
                current_params[j] += change_in_params[j];
            }
        }

        current_params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_function(x: f64, params: &[f64]) -> f64 {
        params[0]  + params[1] * x + params[2] * x.powi(2)
    }

    #[test]
    fn test_curve_fit() {
        let curve_fitter = CurveFit {
            function: test_function,
            max_iterations: 10,
            delta_params: 0.00001,
        };

        let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let params_true = vec![1.0, 2.0, 3.0];

        let y_data: Vec<f64> = x_data.iter()
            .map(|&x| test_function(x, &params_true))
            .collect();

        let initial_params = vec![0.0, 0.0, 0.0];

        let fitted_params = curve_fitter.fit_parameters(
            &x_data, 
            &y_data, 
            &initial_params
        );

        dbg!(&fitted_params);
    }
}