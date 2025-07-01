
use serde::{Deserialize, Serialize};

use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmpiricalCirculationCorrection {
    #[serde(default = "EmpiricalCirculationCorrection::default_exp_factor")]
    pub exp_factor: f64,
}

impl Default for EmpiricalCirculationCorrection {
    fn default() -> Self {
        Self {
            exp_factor: Self::default_exp_factor(),
        }
    }
}

impl EmpiricalCirculationCorrection {
    fn default_exp_factor() -> f64 {10.0}

    pub fn correction_factor(&self, non_dim_span_position: f64) -> f64 {
        let s_in = 0.5 - non_dim_span_position.abs();

        let exp_value = (-self.exp_factor * s_in).exp();
    
        (2.0 / PI) * exp_value.acos()
    }
}