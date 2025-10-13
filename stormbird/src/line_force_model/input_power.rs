

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
pub enum InputPowerModel {
    NoPower,
    FromInternalStateAlone(InputPowerData),
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