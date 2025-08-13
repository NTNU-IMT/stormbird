
use serde::{Deserialize, Serialize};

use stormath::interpolation::linear_interpolation;

use super::prelude::*;

use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinRatioConversion {
    diameter: f64,
    max_rps: f64,
}

impl SpinRatioConversion {
    pub fn get_rps_from_spin_ratio(&self, spin_ratio: f64, velocity: f64) -> f64 {
        let circumference = PI * self.diameter;

        let rps_raw = spin_ratio * velocity / circumference;

        if rps_raw.abs() > self.max_rps {
            self.max_rps * rps_raw.signum()
        } else {
            rps_raw
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum InternalStateType {
    #[default]
    Generic,
    SpinRatio(SpinRatioConversion),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Controller that interpolate on arrays of weather-dependent set points
pub struct WeatherDependentSetPoints {
    pub apparent_wind_directions: Vec<f64>,
    #[serde(default)]
    pub local_angle_of_attack_set_points: Option<Vec<f64>>,
    #[serde(default)]
    pub section_model_internal_state_set_points: Option<Vec<f64>>,
    pub internal_state_type: InternalStateType
}

impl WeatherDependentSetPoints {
    pub fn get_local_angle_of_attack_angle(&self, apparent_wind_direction: f64) -> Option<f64> {
        if let Some(local_angles) = &self.local_angle_of_attack_set_points {
            return Some(
                linear_interpolation(
                    apparent_wind_direction,
                    &self.apparent_wind_directions,
                    local_angles,
                )
            );
        } else {
            return None;
        }
    }

    pub fn get_internal_state(&self, apparent_wind_direction: f64) -> Option<f64> {
        if let Some(internal_states) = &self.section_model_internal_state_set_points {
            return Some(
                linear_interpolation(
                    apparent_wind_direction,
                    &self.apparent_wind_directions,
                    internal_states,
                )
            )
        } else {
            return None
        }
    }

    pub fn get_new_output(&self, input: &ControllerInput) -> ControllerOutput {
        let nr_of_wings = input.apparent_wind_directions.len();

        let local_wing_angles = if self.local_angle_of_attack_set_points.is_some() {
            let mut out = Vec::with_capacity(nr_of_wings);

            for i in 0..nr_of_wings {
                let local_angle_of_attack = input.loading * self.get_local_angle_of_attack_angle(
                    input.apparent_wind_directions[i]
                ).unwrap();

                let wing_angle = input.apparent_wind_directions[i] + local_angle_of_attack;

                out.push(wing_angle);
            }

            Some(out)
        } else {
            None
        };

        let section_models_internal_state = if self.section_model_internal_state_set_points.is_some() {
            let mut out = Vec::with_capacity(nr_of_wings);
            
            for i in 0..nr_of_wings {
                let internal_state_raw = input.loading * self.get_internal_state(
                    input.apparent_wind_directions[i]
                ).unwrap();

                let internal_state = match self.internal_state_type {
                    InternalStateType::Generic => {internal_state_raw}
                    InternalStateType::SpinRatio(ref conversion) => {
                        let velocity = input.velocity[i];

                        conversion.get_rps_from_spin_ratio(internal_state_raw, velocity)
                    }
                };
                
                out.push(internal_state);
            }

            Some(out)

        } else {
            None
        };

        ControllerOutput {
            local_wing_angles: local_wing_angles,
            section_models_internal_state: section_models_internal_state,
        }
    }
}