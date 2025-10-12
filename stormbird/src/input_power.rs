

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::interpolation::linear_interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPower {
    pub section_models_internal_state_data: Vec<Float>,
    pub input_power_per_wing_data: Vec<Float>,
}

impl InputPower {
    pub fn get_input_power(&self, section_models_internal_state: &[Float]) -> Float {
        let mut power = 0.0;

        for wing_index in 0..section_models_internal_state.len() {
            let input_power = linear_interpolation(
                section_models_internal_state[wing_index],
                &self.section_models_internal_state_data,
                &self.input_power_per_wing_data,
            );

            power += input_power;
        }

        power
    }
}