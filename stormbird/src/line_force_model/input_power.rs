// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPowerData {
    pub internal_state_data: Vec<Float>,
    pub input_power_data: Vec<Float>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// An empirical model to calculate the input power required for driving a wind propulsion device.
/// It comes with different modes that represents different ways of calculating the power.
pub enum InputPowerModel {
    /// Default value. Represent a case where a sail does not need power at all, e.g., wing sails
    NoPower,
    /// Assume the internal state is the input power. A simple solution for e.g. suction sails that
    /// is modeled without a fan model
    DirectFromInternalState,
    /// Calculates the power using the internal state. This could, for instance, be a power model 
    /// where the power is calculated directly from the RPS of a rotor sail
    InterpolateFromInternalState(InputPowerData),
}

impl Default for InputPowerModel {
    fn default() -> Self {
        InputPowerModel::NoPower
    }
}

impl InputPowerModel {
    pub fn input_power(&self, internal_state: Float) -> Float {
        match self {
            InputPowerModel::NoPower => 0.0,
            InputPowerModel::DirectFromInternalState => {
                internal_state.abs()
            },
            InputPowerModel::InterpolateFromInternalState(data) => {
                linear_interpolation(
                    internal_state.abs(),
                    &data.internal_state_data,
                    &data.input_power_data,
                )
            },            
        }
    }
}
