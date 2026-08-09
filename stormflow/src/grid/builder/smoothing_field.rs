use stormath::type_aliases::Float;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Holds the parameters for a smoothly-varying 1D field that is `low`
/// almost everywhere and blends up to `high` near each active point.
pub struct SmoothingField {
    pub passive_value: Float,
    pub active_value: Float,
    /// The points locations where the value should be high
    pub active_points: Vec<Float>,
    /// The width of the Gaussian kernel used for each active point. Higher value means more gradual
    /// transitions between high and low values around each active point
    pub gaussian_width: Float,
    /// The main parameter used to control how the softmax smoothing is applied. A high value means
    /// that the softmax is more abrupt - closer to a normal max. Lower value means more "softness"
    pub softmax_strength: Float,
}

impl SmoothingField {
    /// Evaluate the blended field value at a single location `x`.
    pub fn eval(&self, x: Float) -> Float {
        let m = self.active_points.len();

        // 1. gaussian activation from each active point
        let mut g = vec![0.0; m];
        for j in 0..m {
            let dist_sq = (x - self.active_points[j]).powi(2);
            g[j] = (-dist_sq / (2.0 * self.gaussian_width.powi(2))).exp();
        }

        // 2. softmax weights over those activations (smooth max)
        let g_max = g.iter().cloned().fold(Float::NEG_INFINITY, Float::max);
        let mut w = vec![0.0; m];
        for j in 0..m {
            w[j] = (self.softmax_strength * (g[j] - g_max)).exp();
        }
        
        let w_sum: Float = w.iter().sum();
        for j in 0..m {
            w[j] /= w_sum;
        }

        // 3. weighted average of the activations
        let mut blended = 0.0;
        for j in 0..m {
            blended += w[j] * g[j];
        }

        // 4. map blended value in [0, 1] onto [low, high]
        self.passive_value + (self.active_value - self.passive_value) * blended
    }

    /// Convenience method to evaluate the field at many locations at once.
    pub fn eval_many(&self, xs: &[Float]) -> Vec<Float> {
        xs.iter().map(|&x| self.eval(x)).collect()
    }
}