
use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmpiricalCirculationCorrection {
    #[serde(default = "EmpiricalCirculationCorrection::default_exp_factor")]
    pub exp_factor: Float,
    #[serde(default = "EmpiricalCirculationCorrection::default_one")]
    pub overall_correction: Float,
}

impl Default for EmpiricalCirculationCorrection {
    fn default() -> Self {
        Self {
            exp_factor: Self::default_exp_factor(),
            overall_correction: Self::default_one(),
        }
    }
}

impl EmpiricalCirculationCorrection {
    fn default_one() -> Float {1.0}
    fn default_exp_factor() -> Float {10.0}

    pub fn correction_factor(&self, non_dim_span_position: Float) -> Float {
        let s_in = 0.5 - non_dim_span_position.abs();

        let exp_value = (-self.exp_factor * s_in).exp();
    
        self.overall_correction * (2.0 / PI) * exp_value.acos()
    }
}