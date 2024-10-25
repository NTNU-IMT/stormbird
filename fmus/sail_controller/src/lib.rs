use fmu_from_struct::prelude::*;

use stormbird::controllers::sail_controller::SailController as SailControllerInternal;
use stormbird::controllers::sail_controller::SailControllerBuilder;

use std::fs;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct SailController {
    #[parameter]
    pub setup_file_path: String,
    #[input]
    pub angles_of_attack_measurment: String,
    #[output]
    pub local_wing_angles: String,

    controller: Option<SailControllerInternal>,
}

impl FmuFunctions for SailController {
    fn exit_initialization_mode(&mut self) {
        let setup_string = fs::read_to_string(&self.setup_file_path).unwrap();

        let builder: SailControllerBuilder = serde_json::from_str(&setup_string).unwrap();

        self.controller = Some(builder.build());
    }

    fn do_step(&mut self, _current_time: f64, time_step: f64) {
        let angle_measurments: Vec<f64> = if !self.angles_of_attack_measurment.is_empty() {
            serde_json::from_str(&self.angles_of_attack_measurment).unwrap()
        } else {
            vec![0.0; self.nr_wings()]
        };

        let local_wing_angles = if let Some(controller) = &mut self.controller {
            controller.compute_new_wing_angles(
                time_step,
                &angle_measurments
            )
        } else {
            vec![0.0; angle_measurments.len()]
        };

        self.local_wing_angles = serde_json::to_string(&local_wing_angles).unwrap();
    }
}

impl SailController {
    fn nr_wings(&self) -> usize {
        if let Some(controller) = &self.controller {
            controller.target_angles_of_attack.len()
        } else {
            1
        }
    }
}
