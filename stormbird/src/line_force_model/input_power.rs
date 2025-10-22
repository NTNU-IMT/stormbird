// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::interpolation::linear_interpolation;

use crate::line_force_model::span_line::SpanLine;

use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPowerData {
    pub section_models_internal_state_data: Vec<Float>,
    pub input_power_coefficient_data: Vec<Float>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// An empirical model to calculate the input power required for driving a wind propulsion device.
///
/// It comes with different modes. Each mode represents different ways of calculating the data.
pub enum InputPowerModel {
    /// Default value. Represent a case where a sail does not need power at all
    NoPower,
    /// Calculates the power using the internal state of the sectional model alone. This could, for
    /// instance, be a power model where the power is calculated directly from the RPS of a rotor
    /// sail
    FromInternalStateAlone(InputPowerData),
    /// Calculates the power using the both the internal state and the velocity at each section.
    /// This could, for instance, represent a model where the internal state is a non-dimensional
    /// suction rate, so that the actual suction rate, and therefore the power, is dependent on the
    /// velocity squared.
    FromInternalStateAndVelocity(InputPowerData),
}

impl Default for InputPowerModel {
    fn default() -> Self {
        InputPowerModel::NoPower
    }
}

impl InputPowerModel {
    pub fn input_power_coefficient(&self, section_model_internal_state: Float) -> Float {
        match self {
            InputPowerModel::NoPower => 0.0,
            InputPowerModel::FromInternalStateAlone(data) |
            InputPowerModel::FromInternalStateAndVelocity(data) => {
                linear_interpolation(
                    section_model_internal_state.abs(),
                    &data.section_models_internal_state_data,
                    &data.input_power_coefficient_data,
                )
            },
        }
    }

    /// The input power on a given strip, represented by a span lien and chord length.
    pub fn input_power_for_strip(
        &self,
        section_model_internal_state: Float,
        span_line: SpanLine,
        chord_length: Float,
        velocity: SpatialVector
    ) -> Float {
        let power_coefficient = self.input_power_coefficient(section_model_internal_state);

        match self {
            InputPowerModel::NoPower => 0.0,
            InputPowerModel::FromInternalStateAlone(_) => {
                power_coefficient * chord_length * span_line.length()
            },
            InputPowerModel::FromInternalStateAndVelocity(_) => {
                power_coefficient * chord_length * span_line.length() * velocity.length_squared()
            },
        }
    }
}
