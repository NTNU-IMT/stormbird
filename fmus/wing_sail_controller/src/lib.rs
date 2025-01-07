use fmu_from_struct::prelude::*;

use stormbird::controllers::wing_sail::WingSailController as WingSailControllerInternal;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct WingSailController {
    #[parameter]
    pub setup_file_path: String,
    #[input]
    pub loading: f64,
    pub apparent_wind_direction: f64,
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
    pub section_models_internal_state_1: f64,
    pub section_models_internal_state_2: f64,
    pub section_models_internal_state_3: f64,
    pub section_models_internal_state_4: f64,
    pub section_models_internal_state_5: f64,
    pub section_models_internal_state_6: f64,
    pub section_models_internal_state_7: f64,
    pub section_models_internal_state_8: f64,
    pub section_models_internal_state_9: f64,
    pub section_models_internal_state_10: f64,

    controller: Option<WingSailControllerInternal>,
}

impl FmuFunctions for WingSailController {
    fn exit_initialization_mode(&mut self) {
        let setup_file_path = if self.setup_file_path.is_empty() {
            "C:/HLCC 2024 x64/DLL_FMU's/Stormbird/wing_sail_controller_setup.json".to_string() // Default string to facilitate using this FMU in hybrid tests.
        } else {
            self.setup_file_path.clone()
        };

        self.controller = Some(
            WingSailControllerInternal::new_from_file(&setup_file_path)
        );
    }

    fn do_step(&mut self, current_time: f64, time_step: f64) {
        let nr_wings = self.nr_of_wings();

        let angle_measurements = self.get_angle_measurements();

        let local_wing_angles = if let Some(controller) = &mut self.controller {
            controller.loading = self.loading;

            controller.compute_new_wing_angles(
                current_time,
                time_step,
                self.apparent_wind_direction,
                &angle_measurements
            )
        } else {
            vec![0.0; nr_wings]
        };

        let angles_of_attack_estimates = if let Some(controller) = &self.controller {
            controller.angle_of_attack_controller.angle_estimates.clone()
        } else {
            angle_measurements.clone()
        };

        self.set_angle_of_attack_estimates(&angles_of_attack_estimates);

        self.set_local_wing_angles(&local_wing_angles);

        let internal_states = if let Some(controller) = &self.controller {
            controller.new_internal_states(current_time, self.apparent_wind_direction)
        } else {
            vec![0.0; nr_wings]
        };

        self.set_section_models_internal_state(&internal_states);
    }
}

impl WingSailController {
    fn get_angle_measurements(&self) -> Vec<f64> {
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

        let mut angle_measurements = vec![0.0; self.nr_of_wings()];

        for i in 0..self.nr_of_wings() {
            angle_measurements[i] = angle_measurements_raw[i];
        }

        angle_measurements
    }

    fn set_local_wing_angles(&mut self, local_wind_angles: &[f64]) {
        let mut local_wing_angles_extended = vec![0.0; 10];

        for i in 0..self.nr_of_wings() {
            local_wing_angles_extended[i] = local_wind_angles[i];
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

    fn set_angle_of_attack_estimates(&mut self, angles_of_attack_estimates: &[f64]) {
        let mut angle_of_attack_estimates_extended = vec![0.0; 10];

        for i in 0..self.nr_of_wings() {
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
    }

    fn set_section_models_internal_state(&mut self, internal_states: &[f64]) {
        let mut internal_states_extended = vec![0.0; 10];

        for i in 0..self.nr_of_wings() {
            internal_states_extended[i] = internal_states[i];
        }

        self.section_models_internal_state_1  = internal_states_extended[0];
        self.section_models_internal_state_2  = internal_states_extended[1];
        self.section_models_internal_state_3  = internal_states_extended[2];
        self.section_models_internal_state_4  = internal_states_extended[3];
        self.section_models_internal_state_5  = internal_states_extended[4];
        self.section_models_internal_state_6  = internal_states_extended[5];
        self.section_models_internal_state_7  = internal_states_extended[6];
        self.section_models_internal_state_8  = internal_states_extended[7];
        self.section_models_internal_state_9  = internal_states_extended[8];
        self.section_models_internal_state_10 = internal_states_extended[9];
    }

    fn nr_of_wings(&self) -> usize {
        if let Some(controller) = &self.controller {
            controller.nr_of_wings()
        } else {
            1
        }
    }
}
