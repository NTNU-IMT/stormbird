use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// A standard power model for how the wind velocity varies as a function of height
pub struct PowerModel {
    /// The reference velocity for the model. The value is the velocity at the *reference height*, 
    /// which is typically 10 m, but could also be different, depending on the value below
    pub reference_velocity: Float,
    #[serde(default="PowerModel::default_reference_height")]
    /// The reference height for which the reference velocity is valid
    pub reference_height: Float,
    #[serde(default="PowerModel::default_power_factor")]
    /// Factor controlling the shape of the profile. 
    pub power_factor: Float,
}

impl PowerModel {
    /// The wind velocity is often specified at 10 m height in hind cast models
    pub fn default_reference_height() -> Float {10.0}
    /// The ITTC recommends a factor of 1/9 for the power factor if no other information about the
    /// atmospheric boundary layer shape is available.
    pub fn default_power_factor() -> Float {1.0/9.0}
    
    pub fn new_default(reference_velocity: Float) -> Self {
        Self {
            reference_velocity: reference_velocity,
            reference_height: Self::default_reference_height(),
            power_factor: Self::default_power_factor()
        }
    }

    /// The absolute velocity value at the given height
    pub fn velocity_at_height(&self, height: Float) -> Float {
        let increase_factor = self.velocity_increase_factor(height);
        
        self.reference_velocity * increase_factor
    }

    /// Relative value for how much the reference velocity changes at a given height
    pub fn velocity_increase_factor(&self, height: Float) -> Float {
        if self.power_factor > 0.0 {
            (height / self.reference_height).powf(self.power_factor)
        } else {
            1.0
        }
    }
}
