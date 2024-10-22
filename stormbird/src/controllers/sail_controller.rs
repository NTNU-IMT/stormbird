use math_utils::interpolation::linear_interpolation;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Structure for storing and interpolating data that can be used to set the sail state, as a
/// function of the wind direction.
pub struct SailController {
    #[serde(default)]
    pub wind_direction_data: Vec<f64>,
    #[serde(default)]
    pub local_wing_angles_data: Vec<f64>,
    #[serde(default)]
    pub section_model_internal_state_data: Vec<f64>,
}

impl SailController {
    pub fn get_local_wing_angle(&self, wind_direction: f64) -> f64 {
        if self.local_wing_angles_data.len() == 0 {
            return 0.0;
        }

        linear_interpolation(
            wind_direction,
            &self.wind_direction_data,
            &self.local_wing_angles_data,
        )
    }

    pub fn get_section_model_internal_state(&self, wind_direction: f64) -> f64 {
        if self.section_model_internal_state_data.len() == 0 {
            return 0.0;
        }

        linear_interpolation(
            wind_direction,
            &self.wind_direction_data,
            &self.section_model_internal_state_data,
        )
    }
}
