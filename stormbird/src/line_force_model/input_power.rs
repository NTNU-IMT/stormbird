// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::interpolation::linear_interpolation;

use crate::line_force_model::wing_internal_state::{WingInternalState, SPAN_START, SPAN_END};

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
    /// Default value. The power is calculated from the power coefficient of the section models,
    /// integrated over the wing: P = sum(c_P * 0.5 * density * A * U^3). Section models that do
    /// not need power, such as regular foils, have a power coefficient of zero.
    FromSectionModelPowerCoefficient,
    /// Represent a case where a sail does not need power at all, e.g., wing sails
    NoPower,
    /// Assume the internal state is the input power. A simple solution for e.g. suction sails that
    /// is modeled without a fan model. If the internal state varies along the span, it is assumed
    /// to be the power per span length, which is integrated over the span.
    DirectFromInternalState,
    /// Calculates the power using the internal state. This could, for instance, be a power model
    /// where the power is calculated directly from the RPS of a rotor sail. If the internal state
    /// varies along the span, the mean of the absolute value over the span is used.
    InterpolateFromInternalState(InputPowerData),
}

impl Default for InputPowerModel {
    fn default() -> Self {
        InputPowerModel::FromSectionModelPowerCoefficient
    }
}

impl InputPowerModel {
    /// Calculates the input power for a wing.
    ///
    /// # Arguments
    /// * `internal_state` - the internal state of the wing
    /// * `span_length` - the span length of the wing, used if the internal state varies along the
    /// span
    /// * `section_models_power` - function that returns the power integrated from the section
    /// models. Only evaluated when needed.
    pub fn input_power(
        &self,
        internal_state: &WingInternalState,
        span_length: Float,
        section_models_power: impl FnOnce() -> Float
    ) -> Float {
        match self {
            InputPowerModel::FromSectionModelPowerCoefficient => section_models_power(),
            InputPowerModel::NoPower => 0.0,
            InputPowerModel::DirectFromInternalState => {
                match internal_state {
                    WingInternalState::Constant(power) => power.abs(),
                    WingInternalState::SpanVarying(_) => {
                        internal_state.span_integral_of_absolute_value(SPAN_START, SPAN_END, span_length)
                    },
                }
            },
            InputPowerModel::InterpolateFromInternalState(data) => {
                linear_interpolation(
                    internal_state.representative_absolute_value(),
                    &data.internal_state_data,
                    &data.input_power_data,
                )
            },
        }
    }
}
