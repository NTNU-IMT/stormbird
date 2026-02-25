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
/// It comes with different modes that represents different ways of calculating the power.
pub enum InputPowerModel {
    /// Default value. Represent a case where a sail does not need power at all, e.g., wing sails
    NoPower,
    /// Calculates the power using the the internal state as an aerodynamic power power coefficient.
    /// This then represents a model where the internal state is a measure of the input power 
    /// directly, made non-dimensional by the density, sail area, and the velocity to the third 
    /// power. A typical use case would be to model suction sails, where the power coefficient is a 
    /// natural way to represent the internal state of the sail.
    InternalStateAsPowerCoefficient,
    /// Uses the internal state to interpolate an aerodynamic power coefficient from some input 
    /// data. This can be used as an alternative to the `InternalStateAsPowerCoefficient`, to allow
    /// for different values for the internal state and the power coefficient.
    InterpolatePowerCoefficientFromInternalState(InputPowerData),
    /// Calculates the power using the internal state of the sectional model, but not the input 
    /// velocity. This could, for instance, be a power model where the power is calculated directly 
    /// from the RPS of a rotor sail
    InterpolateFromInternalStateOnly(InputPowerData),
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
            InputPowerModel::InternalStateAsPowerCoefficient => {
                section_model_internal_state.abs()
            },
            InputPowerModel::InterpolateFromInternalStateOnly(data) => {
                linear_interpolation(
                    section_model_internal_state.abs(),
                    &data.section_models_internal_state_data,
                    &data.input_power_coefficient_data,
                )
            },
            InputPowerModel::InterpolatePowerCoefficientFromInternalState(data) => {
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
        density: Float,
        velocity: SpatialVector
    ) -> Float {
        let power_coefficient = self.input_power_coefficient(section_model_internal_state);

        match self {
            InputPowerModel::NoPower => 0.0,
            InputPowerModel::InterpolateFromInternalStateOnly(_) => {
                power_coefficient * chord_length * span_line.length()
            },
            InputPowerModel::InternalStateAsPowerCoefficient | 
            InputPowerModel::InterpolatePowerCoefficientFromInternalState(_) => {                
                let dynamic_pressure = 0.5 * density * velocity.length_squared();
                
                let strip_area = chord_length * span_line.length();
                
                power_coefficient * dynamic_pressure * strip_area * velocity.length()
            },
        }
    }
}
