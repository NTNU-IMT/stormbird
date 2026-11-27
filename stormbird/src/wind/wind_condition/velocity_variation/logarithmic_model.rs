use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;

use crate::error::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "LogarithmicModelInput")]
/// Logarithmic model of how velocity varies as a function of height, which also includes optional
/// stability corrections
pub struct LogarithmicModel {
    /// The frictional velocity, defined as the square root of the surface friction from the wind on 
    /// the ground or ocean divided by density. This type of parameter is often available directly 
    /// in hindcast data
    pub friction_velocity: Float,
    /// The surface roughness of the ground or ocean (e.g., how much waves). Can either be computed 
    /// based on the frictional velocity, if some reference velocity is known, or set directly from
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
    /// Coefficient used in the model for a stable stability correction. It is typically fixed for a
    /// given case (e.g., it is not supposed to depend on the weather), so the default value 
    /// should be useful most of the time. However, it is allowed to vary, because different papers
    /// use slight variations here.
    pub stable_coefficient: Float,
    #[serde(default="LogarithmicModel::default_unstable_coefficient")]
    /// Coefficient used in the model for an unstable stability correction. As for the stable 
    /// coefficient above, it is typically fixed, but different papers use slight variations.
    pub unstable_coefficient: Float
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Unchecked version of the logarithmic model, used for deserialization
struct LogarithmicModelInput {
    friction_velocity: Float,
    surface_roughness: Float,
    #[serde(default)]
    obukhov_length: Option<Float>,
    #[serde(default="LogarithmicModel::default_von_karman_constant")]
    von_karman_constant: Float,
    #[serde(default="LogarithmicModel::default_stable_coefficient")]
    stable_coefficient: Float,
    #[serde(default="LogarithmicModel::default_unstable_coefficient")]
    unstable_coefficient: Float
}

impl TryFrom<LogarithmicModelInput> for LogarithmicModel {
    type Error = Error;

    fn try_from(input: LogarithmicModelInput) -> Result<Self, Self::Error> {
        if let Some(obukhov_length) = input.obukhov_length {
            // A length of zero gives a division by zero in the stability correction. A neutral
            // atmosphere is given by leaving the length out, which is the same as an infinite length.
            if obukhov_length == 0.0 || obukhov_length.is_nan() {
                return Err(Error::invalid_input(format!(
                    "The obukhov_length can not be zero or NaN, but is {}. Leave it out for a                     neutral atmosphere.",
                    obukhov_length
                )));
            }
        }

        Ok(Self {
            friction_velocity: input.friction_velocity,
            surface_roughness: input.surface_roughness,
            obukhov_length: input.obukhov_length,
            von_karman_constant: input.von_karman_constant,
            stable_coefficient: input.stable_coefficient,
            unstable_coefficient: input.unstable_coefficient,
        })
    }
}

impl LogarithmicModel {
    /// Default Von Karman constant (0.41)
    pub fn default_von_karman_constant() -> Float {0.41}
    /// Default coefficient for the stable stability correction (6.0)
    pub fn default_stable_coefficient() -> Float {6.0}
    /// Default coefficient for the unstable stability correction (19.3)
    pub fn default_unstable_coefficient() -> Float {19.3}
    
    /// Returns the velocity at the given height, including the stability correction.
    ///
    /// The logarithmic profile is negative below the surface roughness height, which would reverse
    /// the wind. The velocity is therefore limited to zero.
    pub fn velocity_at_height(&self, height: Float) -> Float {
        let velocity = self.neutral_velocity_at_height(height) - self.businger_dyer_correction(height);

        velocity.max(0.0)
    }

    /// Returns the scale factor of the profile, i.e., the friction velocity divided by the Von Karman
    /// constant
    pub fn scale_factor(&self) -> Float {
        self.friction_velocity / self.von_karman_constant
    }
    
    /// Returns the velocity at the given height for a neutral atmosphere. Zero if the height or
    /// the surface roughness is not positive.
    pub fn neutral_velocity_at_height(&self, height: Float) -> Float {
        let undefined_condition = self.surface_roughness <= 0.0 || height <= 0.0;
        
        if undefined_condition {
            return 0.0;
        }
        
        self.scale_factor() * (height / self.surface_roughness).ln()
    }

    /// The equations that compute the Businger Dyer correction to account for non-neutral 
    /// atmosphere.
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
    
    /// Function that computes the Businger-Dyer correction based on the atmosphere state and 
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

    fn model(obukhov_length: Option<Float>) -> LogarithmicModel {
        LogarithmicModel{
            friction_velocity: 0.4,
            surface_roughness: 0.01,
            von_karman_constant: LogarithmicModel::default_von_karman_constant(),
            obukhov_length,
            stable_coefficient: LogarithmicModel::default_stable_coefficient(),
            unstable_coefficient: LogarithmicModel::default_unstable_coefficient()
        }
    }

    #[test]
    /// Below the surface roughness height, the logarithmic profile is negative, which is limited to
    /// zero rather than a reversed wind
    fn velocity_is_not_negative_below_the_surface_roughness() {
        for obukhov_length in [None, Some(100.0), Some(-100.0)] {
            let model = model(obukhov_length);

            assert_eq!(model.velocity_at_height(0.5 * model.surface_roughness), 0.0);
            assert!(model.velocity_at_height(10.0) > 0.0);
        }
    }

    #[test]
    fn zero_obukhov_length_gives_an_error() {
        let result: Result<LogarithmicModel, _> = serde_json::from_str(
            r#"{"friction_velocity": 0.4, "surface_roughness": 0.01, "obukhov_length": 0.0}"#
        );

        assert!(result.is_err());

        let result: Result<LogarithmicModel, _> = serde_json::from_str(
            r#"{"friction_velocity": 0.4, "surface_roughness": 0.01, "obukhov_length": -50.0}"#
        );

        assert!(result.is_ok());
    }
}
