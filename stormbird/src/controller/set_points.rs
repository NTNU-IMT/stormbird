// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use stormath::interpolation::linear_interpolation;

use super::prelude::*;

use stormath::type_aliases::Float;
use stormath::consts::{PI, TAU};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinRatioConversion {
    diameter: Float,
    max_rps: Float,
}

impl SpinRatioConversion {
    pub fn get_rps_from_spin_ratio(&self, spin_ratio: Float, velocity: Float) -> Float {
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

/// Generic function to limit a value to a maximum magnitude
pub fn limit_value(
    old_value: Float,
    raw_new_value: Float,
    max_change: Float,
) -> Float {
    let raw_difference = raw_new_value - old_value;
    
    if raw_difference.abs() > max_change {
        old_value * max_change * raw_difference.signum()
    } else {
        raw_new_value
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Set points for the sail that depends on the apparent wind direction
pub struct ControllerSetPoints {
    pub apparent_wind_directions_data: Vec<Float>,
    #[serde(default)]
    pub angle_of_attack_data: Option<Vec<Float>>,
    #[serde(default)]
    pub section_model_internal_state_data: Option<Vec<Float>>,
    #[serde(default)]
    pub internal_state_type: InternalStateType,
    #[serde(default)]
    pub use_effective_angle_of_attack: bool,
    #[serde(default)]
    pub max_local_wing_angle_change_rate: Option<Float>,
    #[serde(default)]
    pub max_internal_section_state_change_rate: Option<Float>
}

impl ControllerSetPoints {
    pub fn get_new_output(&self, input: &ControllerInput, time_step: Float) -> ControllerOutput {
        let mut local_wing_angle = if self.use_effective_angle_of_attack {
            self.get_local_wing_angle_effective(input)
        } else {
            self.get_local_wing_angle_geometric(input)
        };
        
        if self.max_local_wing_angle_change_rate.is_some() {
            local_wing_angle = limit_value(
                input.current_local_wing_angle, 
                local_wing_angle, 
                self.max_local_wing_angle_change_rate.unwrap() * time_step
            )
        }

        let mut section_model_internal_state = self.get_section_model_internal_state(input);
        
        if self.max_internal_section_state_change_rate.is_some() {
            section_model_internal_state = limit_value(
                input.current_section_model_internal_state, 
                section_model_internal_state, 
                self.max_internal_section_state_change_rate.unwrap() * time_step
            )
        }

        ControllerOutput {
            local_wing_angle,
            section_model_internal_state,
        }
    }

    pub fn get_local_wing_angle_geometric(&self, input: &ControllerInput) -> Float {
        if self.angle_of_attack_data.is_some() {
            let set_point = input.loading * self.get_angle_of_attack_set_point(
                input.apparent_wind_direction
            );

            let wing_angle = input.apparent_wind_direction - set_point;

            wing_angle
        } else {
            0.0
        }
    }

    pub fn get_local_wing_angle_effective(&self, input: &ControllerInput) -> Float {
        let angle_measurement = &input.angle_of_attack;

        if self.angle_of_attack_data.is_some() {
            let set_point = input.loading * self.get_angle_of_attack_set_point(
                input.apparent_wind_direction
            );

            let mut angle_error = angle_measurement - set_point;

            angle_error = Self::correct_angle_to_be_between_pi_and_negative_pi(angle_error);

            let out = angle_measurement + angle_error;

            Self::correct_angle_to_be_between_pi_and_negative_pi(out)
        } else {
            0.0
        }
    }

    pub fn get_section_model_internal_state(&self, input: &ControllerInput) -> Float {
        if self.section_model_internal_state_data.is_some() {
            let internal_state_raw = input.loading * self.get_internal_state_set_point(
                input.apparent_wind_direction
            );

            let internal_state = match self.internal_state_type {
                InternalStateType::Generic => {internal_state_raw}
                InternalStateType::SpinRatio(ref conversion) => {
                    let velocity = input.velocity;

                    conversion.get_rps_from_spin_ratio(internal_state_raw, velocity)
                }
            };

            internal_state
        } else {
            0.0
        }
    }

    pub fn get_angle_of_attack_set_point(&self, apparent_wind_direction: Float) -> Float {
        if let Some(angle_data) = &self.angle_of_attack_data {
            linear_interpolation(
                apparent_wind_direction,
                &self.apparent_wind_directions_data,
                angle_data,
            )
        } else {
            0.0
        }
    }

    pub fn get_internal_state_set_point(&self, apparent_wind_direction: Float) -> Float {
        if let Some(internal_states_data) = &self.section_model_internal_state_data {
            linear_interpolation(
                apparent_wind_direction,
                &self.apparent_wind_directions_data,
                internal_states_data,
            )
        } else {
            0.0
        }
    }

    #[inline(always)]
    fn correct_angle_to_be_between_pi_and_negative_pi(angle: Float) -> Float {
        let mut corrected_angle = angle;

        while corrected_angle > PI {
            corrected_angle -= TAU;
        }
        while corrected_angle < -PI {
            corrected_angle += TAU;
        }

        corrected_angle
    }

    
}
