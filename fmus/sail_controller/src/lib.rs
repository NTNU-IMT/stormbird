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
    pub angle_of_attack_measurement_1: f64,
    pub angle_of_attack_measurement_2: f64,
    pub angle_of_attack_measurement_3: f64,
    pub angle_of_attack_measurement_4: f64,
    pub angle_of_attack_measurement_5: f64,
    pub angle_of_attack_measurement_6: f64,
    pub angle_of_attack_measurement_7: f64,
    pub angle_of_attack_measurement_8: f64,
    pub angle_of_attack_measurement_9: f64,
    pub angle_of_attack_measurement_10: f64,
    #[output]
    pub angle_of_attack_estimate_1: f64,
    pub angle_of_attack_estimate_2: f64,
    pub angle_of_attack_estimate_3: f64,
    pub angle_of_attack_estimate_4: f64,
    pub angle_of_attack_estimate_5: f64,
    pub angle_of_attack_estimate_6: f64,
    pub angle_of_attack_estimate_7: f64,
    pub angle_of_attack_estimate_8: f64,
    pub angle_of_attack_estimate_9: f64,
    pub angle_of_attack_estimate_10: f64,
    pub local_wing_angle_1: f64,
    pub local_wing_angle_2: f64,
    pub local_wing_angle_3: f64,
    pub local_wing_angle_4: f64,
    pub local_wing_angle_5: f64,
    pub local_wing_angle_6: f64,
    pub local_wing_angle_7: f64,
    pub local_wing_angle_8: f64,
    pub local_wing_angle_9: f64,
    pub local_wing_angle_10: f64,

    controller: Option<SailControllerInternal>,
}

impl FmuFunctions for SailController {
    fn exit_initialization_mode(&mut self) {
        let setup_string = fs::read_to_string(&self.setup_file_path).unwrap();

        let builder: SailControllerBuilder = serde_json::from_str(&setup_string).unwrap();

        self.controller = Some(builder.build());
    }

    fn do_step(&mut self, _current_time: f64, time_step: f64) {
        let angle_measurements = self.angle_measurements();

        let local_wing_angles = if let Some(controller) = &mut self.controller {
            controller.compute_new_wing_angles(
                time_step,
                &angle_measurements
            )
        } else {
            vec![0.0; angle_measurements.len()]
        };

        let angles_of_attack_estimates = if let Some(controller) = &self.controller {
            controller.angle_estimate.clone()
        } else {
            angle_measurements.clone()
        };

        let mut angle_of_attack_estimates_extended = vec![0.0; 10];

        for i in 0..self.nr_wings() {
            angle_of_attack_estimates_extended[i] = angles_of_attack_estimates[i];
        }

        self.angle_of_attack_estimate_1  = angle_of_attack_estimates_extended[0];
        self.angle_of_attack_estimate_2  = angle_of_attack_estimates_extended[1];
        self.angle_of_attack_estimate_3  = angle_of_attack_estimates_extended[2];
        self.angle_of_attack_estimate_4  = angle_of_attack_estimates_extended[3];
        self.angle_of_attack_estimate_5  = angle_of_attack_estimates_extended[4];
        self.angle_of_attack_estimate_6  = angle_of_attack_estimates_extended[5];
        self.angle_of_attack_estimate_7  = angle_of_attack_estimates_extended[6];
        self.angle_of_attack_estimate_8  = angle_of_attack_estimates_extended[7];
        self.angle_of_attack_estimate_9  = angle_of_attack_estimates_extended[8];
        self.angle_of_attack_estimate_10 = angle_of_attack_estimates_extended[9];

        let mut local_wing_angles_extended = vec![0.0; 10];

        for i in 0..self.nr_wings() {
            local_wing_angles_extended[i] = local_wing_angles[i];
        }

        self.local_wing_angle_1  = local_wing_angles_extended[0];
        self.local_wing_angle_2  = local_wing_angles_extended[1];
        self.local_wing_angle_3  = local_wing_angles_extended[2];
        self.local_wing_angle_4  = local_wing_angles_extended[3];
        self.local_wing_angle_5  = local_wing_angles_extended[4];
        self.local_wing_angle_6  = local_wing_angles_extended[5];
        self.local_wing_angle_7  = local_wing_angles_extended[6];
        self.local_wing_angle_8  = local_wing_angles_extended[7];
        self.local_wing_angle_9  = local_wing_angles_extended[8];
        self.local_wing_angle_10 = local_wing_angles_extended[9];
    }
}

impl SailController {
    fn angle_measurements(&self) -> Vec<f64> {
        let angle_measurements_raw = vec![
            self.angle_of_attack_measurement_1, 
            self.angle_of_attack_measurement_2, 
            self.angle_of_attack_measurement_3, 
            self.angle_of_attack_measurement_4, 
            self.angle_of_attack_measurement_5, 
            self.angle_of_attack_measurement_6, 
            self.angle_of_attack_measurement_7, 
            self.angle_of_attack_measurement_8, 
            self.angle_of_attack_measurement_9, 
            self.angle_of_attack_measurement_10
        ];

        let mut angle_measurements = vec![0.0; self.nr_wings()];

        for i in 0..self.nr_wings() {
            angle_measurements[i] = angle_measurements_raw[i];
        }

        angle_measurements
    }

    fn nr_wings(&self) -> usize {
        if let Some(controller) = &self.controller {
            controller.target_angles_of_attack.len()
        } else {
            1
        }
    }
}
