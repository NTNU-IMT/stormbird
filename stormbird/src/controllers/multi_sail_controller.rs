use super::sail_controller::SailController;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MultiSailController {
    pub controllers: Vec<SailController>,
}

impl MultiSailController {
    pub fn get_local_wing_angles(&self, wind_direction: f64) -> Vec<f64> {
        let mut local_wing_angles = Vec::with_capacity(self.controllers.len());

        for controller in &self.controllers {
            local_wing_angles.push(controller.get_local_wing_angle(wind_direction));
        }

        local_wing_angles
    }

    pub fn get_section_models_internal_state(&self, wind_direction: f64) -> Vec<f64> {
        let mut section_models_internal_state = Vec::with_capacity(self.controllers.len());

        for controller in &self.controllers {
            section_models_internal_state
                .push(controller.get_section_model_internal_state(wind_direction));
        }

        section_models_internal_state
    }
}
