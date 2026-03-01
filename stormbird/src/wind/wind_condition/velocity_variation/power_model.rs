use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PowerModel {
    pub reference_velocity: Float,
    #[serde(default="PowerModel::default_reference_height")]
    pub reference_height: Float,
    #[serde(default="PowerModel::default_power_factor")]
    pub power_factor: Float,
}

impl PowerModel {
    pub fn default_reference_height() -> Float {10.0}
    pub fn default_power_factor() -> Float {1.0/9.0}
    
    pub fn new_default(reference_velocity: Float) -> Self {
        Self {
            reference_velocity: reference_velocity,
            reference_height: Self::default_reference_height(),
            power_factor: Self::default_power_factor()
        }
    }
    
    pub fn velocity_at_height(&self, height: Float) -> Float {
        let increase_factor = self.velocity_increase_factor(height);
        
        self.reference_velocity * increase_factor
    }

    pub fn velocity_increase_factor(&self, height: Float) -> Float {
        if self.power_factor > 0.0 {
            (height / self.reference_height).powf(self.power_factor)
        } else {
            1.0
        }
    }
}