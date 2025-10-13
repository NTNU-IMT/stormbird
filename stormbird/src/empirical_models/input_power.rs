

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPowerFromInternalStateAlone {
    pub section_models_internal_state_data: Vec<Float>,
    pub input_power_divided_by_area: Vec<Float>,
}

impl InputPowerFromInternalStateAlone {
    pub fn get_input_power(&self, section_model_internal_state: &[Float], area: Float) -> Float {
        todo!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPowerFromInternalStateAndWindVelocity {
    pub section_models_internal_state_data: Vec<Float>,
    pub non_dim_input_power: Vec<Vec<Float>>,
}