use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LogarithmicModel {
    pub friction_velocity: Float,
    pub surface_roughness: Float,
    #[serde(default="LogarithmicModel::default_von_karman_constant")]
    pub von_karman_constant: Float,
    #[serde(default)]
    pub obukhov_length: Option<Float>,
}

impl LogarithmicModel {
    pub fn default_von_karman_constant() -> Float {0.41}
    
    pub fn velocity_at_height(&self, height: Float) -> Float {
        self.neutral_velocity_at_height(height) - self.businger_dyer_correction(height)
    }

    pub fn scale_factor(&self) -> Float {
        self.friction_velocity / self.von_karman_constant
    }
    
    pub fn neutral_velocity_at_height(&self, height: Float) -> Float {
        let undefined_condition = self.surface_roughness <= 0.0 || height <= 0.0;
        
        if undefined_condition {
            return 0.0;
        }
        
        self.scale_factor() * (height / self.surface_roughness).ln()
    }
    
    pub fn businger_dyer_unscaled_correction(&self, height: Float) -> Float {
        if let Some(length) = self.obukhov_length {         
            let zeta = height / length;
            
            if length > 0.0 {
                // Stable
                -5.0 * zeta
            } else {
                // Unstable
                let x = (1.0 - 16.0 * zeta).powf(0.25);
                
                let first_term = 2.0 * ((1.0 + x) / 2.0).ln();
                let second_term = ((1.0 + x.powi(2)) / 2.0).ln();
                let third_term = -2.0 * x.atan() + PI / 2.0;
                
                first_term + second_term + third_term
            }
        } else {
            0.0
        }
    }
    
    /// Function that computer the Businger-Dyer correction based on the atmosphere state and 
    /// the Obukhov length
    pub fn businger_dyer_correction(&self, height: Float) -> Float {
        let non_scaled_correction = self.businger_dyer_unscaled_correction(height);
        
        non_scaled_correction * self.scale_factor()
    }
}
