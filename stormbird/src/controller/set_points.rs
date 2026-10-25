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
/// Defines the units of the internal state set point data
pub enum InternalStateSetPointType {
    #[default]
    /// The data is given in the units of the section model, e.g., spin ratio for rotor sails or 
    /// the suction coefficient for suction sails, and is converted to the wing internal state.
    SectionModel,
    /// The data is given directly as the wing internal state, e.g., the power (with the sign 
    /// setting the direction) for suction sails, and is used without conversion.
    Wing,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Set points for the sail that depends on the apparent wind direction
pub struct ControllerSetPoints {
    /// The apparent wind directions where the set points are given.
    ///
    /// **Note**: the controller measures the apparent wind direction for each wing as the angle from
    /// the chord to the velocity, around the span direction of the wing. This is not the same as in
    /// the wind environment, which measures it around the `wind_rotation_axis`. For sails built from
    /// the bottom to the top, with the default wind environment, the two have opposite signs, so a
    /// wind from starboard is negative here. Set points that are mirrored between positive and
    /// negative directions are not affected. A fix is targeted for the next release.
    pub apparent_wind_directions_data: Vec<Float>,
    /// Optional angle of attack set points at each apparent wind direction
    #[serde(default)]
    pub angle_of_attack_data: Option<Vec<Float>>,
    #[serde(default)]
    /// Internal state set points, given in the units specified by `internal_state_set_point_type`
    pub internal_state_data: Option<Vec<Float>>,
    #[serde(default)]
    /// Defines the units of the internal state set point data
    pub internal_state_set_point_type: InternalStateSetPointType,
    #[serde(default)]
    /// Maximum absolute value of the wing internal state, applied after any conversion from the 
    /// set point data. The sign of the internal state is preserved. The meaning depends on the 
    /// section model: e.g., revolutions per second for rotor sails, or the power for suction 
    /// sails where the wing state is the power. For suction sails using 
    /// `SuctionCoefficientDirect`, the wing state, and therefore this value, is the suction 
    /// coefficient.
    pub wing_internal_state_max_value: Option<Float>,
    /// If true, the local wing angle is set from the measured effective angle of attack. If false,
    /// it is set from the geometric angle between the apparent wind and the wing.
    #[serde(default)]
    pub use_effective_angle_of_attack: bool,
    /// Optional maximum change rate of the local wing angle, per unit time
    #[serde(default)]
    pub max_local_wing_angle_change_rate: Option<Float>,
    #[serde(default)]
    /// Maximum change rate of the wing internal state, per unit time. Applied after any
    /// conversion from the set point data, so the units follow the wing internal state, e.g.,
    /// power per second for suction sails where the wing state is the power.
    pub wing_internal_state_max_change_rate: Option<Float>
}

impl ControllerSetPoints {
    /// Returns the new local wing angle, limited by the maximum change rate if given
    pub fn get_local_wing_angle(&self, input: &ControllerInput, time_step: Float) -> Float {
        let mut local_wing_angle = if self.use_effective_angle_of_attack {
            self.get_local_wing_angle_effective(input)
        } else {
            self.get_local_wing_angle_geometric(input)
        };
        
        if let Some(max_change_rate) = self.max_local_wing_angle_change_rate {
            local_wing_angle = limit_value(
                input.current_local_wing_angle,
                local_wing_angle,
                max_change_rate * time_step
            )
        }

        local_wing_angle
    }

    /// Returns the angle of attack set point at the given apparent wind direction, or zero if no
    /// angle of attack data is given
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

    /// Returns the local wing angle that gives the angle of attack set point, based on the geometric
    /// angle between the apparent wind and the wing
    pub fn get_local_wing_angle_geometric(&self, input: &ControllerInput) -> Float {
        if self.angle_of_attack_data.is_some() {
            let set_point = input.loading * self.get_angle_of_attack_set_point(
                input.apparent_wind_direction
            );

            

            input.apparent_wind_direction - set_point
        } else {
            0.0
        }
    }

    /// Returns the local wing angle that gives the angle of attack set point, based on the error in
    /// the measured effective angle of attack
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
    

    /// Returns the new internal state of the wing, limited by the maximum value and change rate if
    /// given
    pub fn get_internal_state(
        &self,
        wing_index: usize,
        line_force_model: &LineForceModel,
        input: &ControllerInput,
        time_step: Float
    ) -> Float {
        if self.internal_state_data.is_some() {
            let set_point = input.loading * self.get_internal_state_set_point(
                input.apparent_wind_direction
            );

            let mut internal_state = match self.internal_state_set_point_type {
                InternalStateSetPointType::Wing => set_point,
                InternalStateSetPointType::SectionModel => Self::wing_state_from_section_state(
                    set_point, wing_index, line_force_model, input
                ),
            };

            if let Some(max_value) = self.wing_internal_state_max_value {
                internal_state = internal_state.abs().min(max_value) * internal_state.signum();
            }

            if let Some(max_change_rate) = self.wing_internal_state_max_change_rate {
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

    /// Converts an internal state given in the units of the section model to the wing internal
    /// state, using the measured flow conditions as representative for the whole wing.
    pub fn wing_state_from_section_state(
        section_model_internal_state: Float,
        wing_index: usize,
        line_force_model: &LineForceModel,
        input: &ControllerInput,
    ) -> Float {
        match &line_force_model.section_models[wing_index] {
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
            SectionModel::ActiveAirFlowFoil(active_air_flow_foil) => {
                // TODO: find a more elegant way of reading the right projected area
                let area = line_force_model.projected_areas()[wing_index];

                active_air_flow_foil.wing_state_from_section_state(
                    section_model_internal_state,
                    input.velocity,
                    area,
                    line_force_model.internal_compartments[wing_index].nr_compartments(),
                    line_force_model.density
                )
            }
        }
    }

    

    /// Returns the internal state set point at the given apparent wind direction, or zero if no
    /// internal state data is given
    pub fn get_internal_state_set_point(&self, apparent_wind_direction: Float) -> Float {
        if let Some(internal_states_data) = &self.internal_state_data {
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
