
use serde::{Deserialize, Serialize};
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherDependentSetpoints {
    pub apparent_wind_directions: Vec<f64>,
    pub effective_angles_of_attack: Vec<f64>,
    pub internal_states: Vec<f64>,
}

impl WeatherDependentSetpoints {
    pub fn get_effective_angle_of_attack(&self, apparent_wind_direction: f64) -> f64 {
        linear_interpolation(
            apparent_wind_direction,
            &self.apparent_wind_directions,
            &self.effective_angles_of_attack,
        )
    }

    pub fn get_internal_state(&self, apparent_wind_direction: f64) -> f64 {
        linear_interpolation(
            apparent_wind_direction,
            &self.apparent_wind_directions,
            &self.internal_states,
        )
    }   
}