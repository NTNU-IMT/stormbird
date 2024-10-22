use fmu_from_struct::prelude::*;

use stormbird::controllers::multi_sail_controller::MultiSailController;

use std::fs;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct SailController {
    #[parameter]
    pub setup_file_path: String,
    #[input]
    pub wind_direction: f64,
    #[output]
    pub local_wing_angles: String,
    pub section_models_internal_state: String,

    controller: Option<MultiSailController>,
}

impl FmuFunctions for SailController {
    fn exit_initialization_mode(&mut self) {
        let setup_string = fs::read_to_string(&self.setup_file_path).unwrap();

        let controller: MultiSailController = serde_json::from_str(&setup_string).unwrap();

        self.controller = Some(controller);
    }

    fn do_step(&mut self, _current_time: f64, _time_step: f64) {
        if let Some(controller) = &self.controller {
            let local_wing_angles = controller.get_local_wing_angles(self.wind_direction);
            let section_models_internal_state =
                controller.get_section_models_internal_state(self.wind_direction);

            self.local_wing_angles = serde_json::to_string(&local_wing_angles).unwrap();

            self.section_models_internal_state =
                serde_json::to_string(&section_models_internal_state).unwrap();
        }
    }
}
