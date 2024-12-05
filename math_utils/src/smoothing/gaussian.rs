use super::*;

/// Gaussian kernel to be used in smoothing function.
/// Based on: <https://en.wikipedia.org/wiki/Kernel_smoother>
pub fn gaussian_kernel(x: f64, x0: f64, smoothing_length: f64) -> f64 {
    let x_prime = x - x0;

    (-x_prime.powi(2) / (2.0 * smoothing_length.powi(2))).exp()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GaussianSmoothing {
    smoothing_length: f64,
    number_of_end_insertions: usize,
    end_conditions: [EndCondition; 2]
}

impl GaussianSmoothing {
    /// Gaussian smoothing using the kernel function above.
    /// Based on: <https://en.wikipedia.org/wiki/Kernel_smoother>
    pub fn apply_smoothing<T>(&self, x: &[f64], y: &[T]) -> Vec<T>
    where T: SmoothingOps
    {
        let x_modified = EndCondition::add_end_values_to_x_data(x, self.number_of_end_insertions);
        let y_modified = EndCondition::add_end_values_to_y_data(y, self.number_of_end_insertions, self.end_conditions);

        let n = y.len();
        let n_mod = y_modified.len();

        let mut y_smooth: Vec<T> = Vec::with_capacity(n);

        // i_0 represents the index of the point to be smoothed
        for i_0 in 0..n {
            let mut kernel_sum = 0.0;
            let mut kernel_func_product_sum: T = Default::default();

            for i in 0..n_mod {

                let kernel_value = gaussian_kernel(x_modified[i], x[i_0], self.smoothing_length);

                kernel_sum += kernel_value;
                kernel_func_product_sum = kernel_func_product_sum + y_modified[i] * kernel_value;
            }

            y_smooth.push(kernel_func_product_sum / kernel_sum);
            
        }

        y_smooth
    }
}



