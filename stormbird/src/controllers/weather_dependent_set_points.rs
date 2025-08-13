
use serde::{Deserialize, Serialize};

use stormath::interpolation::linear_interpolation;

use super::prelude::*;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum InternalStateType {
    #[default]
    Generic,
    SpinRatio(f64),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Controller that interpolate on arrays of weather-dependent set points
pub struct WeatherDependentSetPoints {
    pub apparent_wind_directions: Vec<f64>,
    pub local_wing_angle_set_points: Vec<f64>,
    pub section_model_internal_state_set_points: Vec<f64>,
    pub internal_state_type: InternalStateType
}

impl WeatherDependentSetPoints {
    pub fn get_local_wing_angle(&self, apparent_wind_direction: f64) -> f64 {
        linear_interpolation(
            apparent_wind_direction,
            &self.apparent_wind_directions,
            &self.local_wing_angle_set_points,
        )
    }

    pub fn get_internal_state(&self, apparent_wind_direction: f64) -> f64 {
        linear_interpolation(
            apparent_wind_direction,
            &self.apparent_wind_directions,
            &self.section_model_internal_state_set_points,
        )
    }

    pub fn get_new_output(
        &self,
        apparent_wind_directions: &[f64]
    ) -> ControllerOutput {
        let nr_of_wings = apparent_wind_directions.len();

        let mut local_wing_angles = Vec::with_capacity(nr_of_wings);
        let mut section_models_internal_state = Vec::with_capacity(nr_of_wings);

        for i in 0..nr_of_wings {
            local_wing_angles.push(
                self.get_local_wing_angle(apparent_wind_directions[i])
            );
            
            section_models_internal_state.push(
                self.get_internal_state(apparent_wind_directions[i])
            );
        }

        ControllerOutput {
            local_wing_angles: Some(local_wing_angles),
            section_models_internal_state: Some(section_models_internal_state),
        }
    }
}