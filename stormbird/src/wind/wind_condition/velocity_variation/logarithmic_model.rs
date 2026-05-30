use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Logarithmic model of how velocity varies as a function of height, which also includes optional
/// stability corrections
pub struct LogarithmicModel {
    /// The frictional velocity, defined as the square root of the surface friction from the wind on 
    /// the ground or ocean, divided by density. This type of parameter is often available directly 
    /// in hindcast data
    pub friction_velocity: Float,
    /// The surface roughness of the ground or ocean (e.g., how much waves). Can either be computed 
    /// based if some reference velocity and the frictional velocity is known, or set directly from
    /// hindcast data
    pub surface_roughness: Float,
    #[serde(default)]
    /// The [Obukhov length](https://en.wikipedia.org/wiki/Monin%E2%80%93Obukhov_length) is used to 
    /// correct the profile for non-neutral conditions, e.g., stable or unstable atmospheres.
    pub obukhov_length: Option<Float>,
    #[serde(default="LogarithmicModel::default_von_karman_constant")]
    /// The [Von Karman constant](https://en.wikipedia.org/wiki/Von_K%C3%A1rm%C3%A1n_constant) 
    pub von_karman_constant: Float,
    #[serde(default="LogarithmicModel::default_stable_coefficient")]
    pub stable_coefficient: Float,
    #[serde(default="LogarithmicModel::default_unstable_coefficient")]
    pub unstable_coefficient: Float
}

impl LogarithmicModel {
    pub fn default_von_karman_constant() -> Float {0.41}
    pub fn default_stable_coefficient() -> Float {6.0}
    pub fn default_unstable_coefficient() -> Float {19.3}
    
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
                -self.stable_coefficient * zeta
            } else {
                // Unstable
                let x = (1.0 - self.unstable_coefficient* zeta).powf(0.25);
                
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    /// Test that the unscaled Businger-Dyer correction equals zero when the height is also zero
    fn test_businger_dyer_unscaled_correction_at_zero_height() {
        let model = LogarithmicModel{
            friction_velocity: 1.0,
            surface_roughness: 0.001,
            von_karman_constant: LogarithmicModel::default_von_karman_constant(),
            obukhov_length: Some(10.0),
            stable_coefficient: LogarithmicModel::default_stable_coefficient(),
            unstable_coefficient: LogarithmicModel::default_unstable_coefficient()
        };
        
        let test_correction_value = model.businger_dyer_unscaled_correction(0.0).abs();
        
        dbg!(test_correction_value);
        assert!(
            test_correction_value < 1e-6, 
            "Value is the unscaled Businger Dyer correction is significantly larger than zero"
        );
    }
}
