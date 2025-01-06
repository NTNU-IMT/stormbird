pub mod weather_dependent_setpoints;
pub mod angle_of_attack_controller;

use serde::{Deserialize, Serialize};

use angle_of_attack_controller::{AngleOfAttackControllerBuilder, AngleOfAttackController};
use weather_dependent_setpoints::WeatherDependentSetpoints;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WingSailControllerBuilder {
    pub angle_of_attack_controller: AngleOfAttackControllerBuilder,
    pub weather_dependent_setpoints: WeatherDependentSetpoints,
}

impl WingSailControllerBuilder {
    pub fn build(&self) -> WingSailController {
        WingSailController {
            loading: 1.0,
            angle_of_attack_controller: self.angle_of_attack_controller.build(),
            weather_dependent_setpoints: self.weather_dependent_setpoints.clone(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct WingSailController {
    pub loading: f64,
    pub angle_of_attack_controller: AngleOfAttackController,
    pub weather_dependent_setpoints: WeatherDependentSetpoints,
}

impl WingSailController {
    pub fn new_from_file(file_path: &str) -> Self {
        let setup_string = std::fs::read_to_string(file_path).unwrap();

        let builder: WingSailControllerBuilder = serde_json::from_str(&setup_string).unwrap();

        builder.build()
    }

    pub fn compute_new_wing_angles(
        &mut self, 
        time_step: f64,
        apparent_wind_direction: f64, 
        angle_measurements: &[f64]
    ) -> Vec<f64> {
        let nr_of_wings = self.angle_of_attack_controller.nr_of_wings();

        let target_angle_of_attack = 
            self.weather_dependent_setpoints.get_effective_angle_of_attack(apparent_wind_direction);

        let target_angles_of_attack = vec![target_angle_of_attack * self.loading; nr_of_wings];

        self.angle_of_attack_controller.compute_new_wing_angles(
            time_step,
            &target_angles_of_attack,
            angle_measurements,
        )
    }

    pub fn new_internal_states(
        &self,
        apparent_wind_direction: f64,
    ) -> Vec<f64> {
        let nr_of_wings = self.angle_of_attack_controller.nr_of_wings();

        let internal_state = 
            self.weather_dependent_setpoints.get_internal_state(apparent_wind_direction);

        vec![internal_state * self.loading; nr_of_wings]
    }

    pub fn nr_of_wings(&self) -> usize {
        self.angle_of_attack_controller.nr_of_wings()
    }
}