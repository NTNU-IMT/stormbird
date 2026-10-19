// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use stormath::interpolation::linear_interpolation;

use super::prelude::*;
use crate::line_force_model::LineForceModel;
use crate::section_models::SectionModel;
use crate::section_models::rotating_cylinder::RotatingCylinder;

use stormath::type_aliases::Float;
use stormath::consts::{PI, TAU};

/// Generic function to limit a value to a maximum magnitude
pub fn limit_value(
    old_value: Float,
    raw_new_value: Float,
    max_change: Float,
) -> Float {
    let raw_difference = raw_new_value - old_value;
    
    if raw_difference.abs() > max_change {
        old_value + max_change * raw_difference.signum()
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
    pub internal_state_max_value: Option<Float>,
    #[serde(default)]
    pub use_effective_angle_of_attack: bool,
    #[serde(default)]
    pub max_local_wing_angle_change_rate: Option<Float>,
    #[serde(default)]
    pub max_internal_state_change_rate: Option<Float>
}

impl ControllerSetPoints {
    pub fn get_local_wing_angle(&self, input: &ControllerInput, time_step: Float) -> Float {
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

        local_wing_angle
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

            let out = input.current_local_wing_angle + angle_error;

            Self::correct_angle_to_be_between_pi_and_negative_pi(out)
        } else {
            0.0
        }
    }
    

    pub fn get_internal_state(
        &self,
        wing_index: usize,
        line_force_model: &LineForceModel,
        input: &ControllerInput,
        time_step: Float
    ) -> Float {
        if self.section_model_internal_state_data.is_some() {
            let section_model_internal_state = input.loading * self.get_section_model_internal_state(
                input.apparent_wind_direction
            );

            // Convert the section internal state to internal state
            let mut internal_state = match &line_force_model.section_models[wing_index] {
                SectionModel::SingleElementFoil(_) | SectionModel::MultiElementFoil(_) | SectionModel::EffectiveWindSensor => {
                    section_model_internal_state
                },
                SectionModel::RotatingCylinder(_) => {
                    // TODO: find a more elegant way of getting the diameter
                    let diameter = line_force_model.chord_lengths[line_force_model.wing_indices[wing_index].clone()][0];

                    RotatingCylinder::revolutions_per_second_from_spin_ratio(
                        section_model_internal_state, 
                        diameter, 
                        input.velocity
                    )
                },
                SectionModel::ActiveAirFlowFoil(_) => {
                    // TODO: find a more elegant way of reading the right projected area
                    let area = line_force_model.projected_areas()[wing_index];

                    section_model_internal_state * 0.5 * area * line_force_model.density * input.velocity.powi(3)
                }
            };

            if let Some(max_value) = self.internal_state_max_value {
                internal_state = internal_state.abs().min(max_value) * internal_state.signum();
            }

            if let Some(max_change_rate) = self.max_internal_state_change_rate {
                internal_state = limit_value(
                    input.current_internal_state, 
                    internal_state, 
                    max_change_rate * time_step
                )
            }

            internal_state
        } else {
            0.0
        }
    }

    

    pub fn get_section_model_internal_state(&self, apparent_wind_direction: Float) -> Float {
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
