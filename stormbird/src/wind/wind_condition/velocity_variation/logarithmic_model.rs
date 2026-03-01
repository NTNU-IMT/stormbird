use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum AtmosphereState {
    #[default]
    Neutral,
    Stable,
    Unstable
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LogarithmicModel {
    pub friction_velocity: Float,
    pub surface_roughness: Float,
    #[serde(default="LogarithmicModel::default_von_karman_constant")]
    pub von_karman_constant: Float,
    #[serde[default]]
    pub obukhov_length: Option<Float>,
    #[serde(default)]
    pub atmosphere_state: AtmosphereState
}

impl LogarithmicModel {
    pub fn default_von_karman_constant() -> Float {0.41}
    
    pub fn velocity_at_height(&self, height: Float) -> Float {
        self.neutral_velocity_at_height(height) + self.businger_dyer_correction(height)
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
    
    /// Function that computer the Businger-Dyer correction based on the atmosphere state and 
    /// the Obukhov length
    pub fn businger_dyer_correction(&self, height: Float) -> Float {
        let non_scaled_correction = if let Some(length) = self.obukhov_length {
            let zeta = height / length;
            
            match self.atmosphere_state {
                AtmosphereState::Neutral => 0.0,
                AtmosphereState::Unstable => {
                    let x = (1.0 - 16.0 * zeta).powf(1.0/4.0);
                    
                    let first_term = 2.0 * ((1.0 + x) / 2.0).ln();
                    let second_term = ((1.0 + x.powi(2))).ln();
                    let third_term = -2.0 * x.atan() + PI / 2.0;
                    
                    first_term + second_term + third_term
                }
                AtmosphereState::Stable => {
                    -5.0 * zeta
                }
            }
        } else {
            0.0
        };
        
        non_scaled_correction * self.scale_factor()
    }
}