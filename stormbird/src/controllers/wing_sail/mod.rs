pub mod weather_dependent_setpoints;

use serde::{Deserialize, Serialize};

use angle_of_attack_controller::{AngleOfAttackControllerBuilder, AngleOfAttackController};
use weather_dependent_setpoints::WeatherDependentSetpoints;

use crate::error::Error;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WingSailControllerBuilder {
    pub angle_of_attack_controller: AngleOfAttackControllerBuilder,
    pub weather_dependent_setpoints: WeatherDependentSetpoints,
    #[serde(default)]
    pub start_at_zero_time: f64,
    #[serde(default = "WingSailControllerBuilder::default_model_scale_factor")]
    pub model_scale_factor: f64,
}

impl WingSailControllerBuilder {
    pub fn default_model_scale_factor() -> f64 {1.0}

    pub fn build(&self) -> WingSailController {
        WingSailController {
            loading: 1.0,
            angle_of_attack_controller: self.angle_of_attack_controller.build(),
            weather_dependent_setpoints: self.weather_dependent_setpoints.clone(),
            start_at_zero_time: self.start_at_zero_time,
            model_scale_factor: self.model_scale_factor,
        }
    }

    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let setup_string = std::fs::read_to_string(file_path)?;

        let builder: WingSailControllerBuilder = serde_json::from_str(&setup_string)?;

        Ok(builder)
    }
}

#[derive(Debug, Default, Clone)]
pub struct WingSailController {
    pub loading: f64,
    pub angle_of_attack_controller: AngleOfAttackController,
    pub weather_dependent_setpoints: WeatherDependentSetpoints,
    pub start_at_zero_time: f64,
    pub model_scale_factor: f64,
}

impl WingSailController {
    pub fn upscaled_time_value(&self, time_value: f64) -> f64 {
        if self.model_scale_factor > 0.0 {
            time_value / self.model_scale_factor.sqrt()
        } else {
            time_value
        }
    }

    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let builder = WingSailControllerBuilder::new_from_file(file_path)?;

        Ok(builder.build())
    }

    pub fn compute_new_wing_angles(
        &mut self, 
        time_in: f64,
        time_step_in: f64,
        apparent_wind_direction: f64, 
        angle_measurements: &[f64]
    ) -> Vec<f64> {
        let time = self.upscaled_time_value(time_in);
        let nr_of_wings = self.angle_of_attack_controller.nr_of_wings();

        let time_step = self.upscaled_time_value(time_step_in);

        let target_angle_of_attack = if time < self.start_at_zero_time {
            0.0
        } else {
            self.weather_dependent_setpoints.get_effective_angle_of_attack(apparent_wind_direction)
        };

        let target_angles_of_attack = vec![target_angle_of_attack * self.loading; nr_of_wings];

        self.angle_of_attack_controller.compute_new_wing_angles(
            time_step,
            &target_angles_of_attack,
            angle_measurements,
        )
    }

    pub fn new_internal_states(
        &self,
        time_in: f64,
        apparent_wind_direction: f64,
    ) -> Vec<f64> {
        let time = self.upscaled_time_value(time_in);
        let nr_of_wings = self.angle_of_attack_controller.nr_of_wings();

        if time < self.start_at_zero_time {
            return vec![0.0; nr_of_wings];
        }

        let internal_state = 
            self.weather_dependent_setpoints.get_internal_state(apparent_wind_direction);

        vec![internal_state * self.loading; nr_of_wings]
    }

    pub fn nr_of_wings(&self) -> usize {
        self.angle_of_attack_controller.nr_of_wings()
    }
}