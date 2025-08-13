// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of various controllers for wind propulsion devices intended to be used together
//! with the simulation models in the library. 

pub mod input;
pub mod output;
pub mod measurements;
pub mod weather_dependent_set_points;
pub mod effective_angle_of_attack;
//pub mod pid_controller;
pub mod prelude;

use crate::error::Error;

use serde::{Deserialize, Serialize};

use input::ControllerInput;
use output::ControllerOutput;
use measurements::FlowMeasurementSettings;

use effective_angle_of_attack::{
    EffectiveAngleOfAttackController,
};

use weather_dependent_set_points::{
    WeatherDependentSetPoints
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControllerLogic {
    EffectiveAngleOfAttack(EffectiveAngleOfAttackController),
    WeatherDependentSetPoints(WeatherDependentSetPoints),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerBuilder {
    pub logic: ControllerLogic,
    #[serde(default)]
    pub flow_measurement_settings: FlowMeasurementSettings,
    #[serde(default = "ControllerBuilder::default_time_steps_between_updates")]
    pub time_steps_between_updates: usize,
    #[serde(default)]
    pub start_time: f64,
    #[serde(default)]
    pub max_local_wing_angle_change_rate: Option<f64>,
    #[serde(default)]
    pub max_internal_section_state_change_rate: Option<f64>,
    #[serde(default)]
    pub moving_average_window_size: Option<usize>,
    #[serde(default = "ControllerBuilder::default_update_factor")]
    pub update_factor: f64,
}

impl ControllerBuilder {
    pub fn default_time_steps_between_updates() -> usize {1}
    pub fn default_update_factor() -> f64 {1.0}

    pub fn from_json_string(json_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(json_string)?;

        Ok(serde_res)
    }

    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;
        
        Self::from_json_string(&json_string)
    }

    pub fn build(&self) -> Controller {
        Controller {
            logic: self.logic.clone(),
            flow_measurement_settings: self.flow_measurement_settings.clone(),
            time_steps_between_updates: self.time_steps_between_updates,
            start_time: self.start_time,
            max_local_wing_angle_change_rate: self.max_local_wing_angle_change_rate,
            max_internal_section_state_change_rate: self.max_internal_section_state_change_rate,
            update_factor: self.update_factor,
            time_step_index: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub logic: ControllerLogic,
    pub flow_measurement_settings: FlowMeasurementSettings,
    pub time_steps_between_updates: usize,
    pub start_time: f64,
    pub max_local_wing_angle_change_rate: Option<f64>,
    pub max_internal_section_state_change_rate: Option<f64>,
    pub update_factor: f64,
    pub time_step_index: usize
}


impl Controller {
    pub fn update(
        &self,
        time: f64,
        time_step: f64, 
        input: &ControllerInput,
    ) -> Option<ControllerOutput> {
        let initialization_done = time >= self.start_time;
        let time_to_update =  self.time_step_index % self.time_steps_between_updates == 0;
        let first_time_step = self.time_step_index == 1;

        if first_time_step || (time_to_update && initialization_done) {
            let new_output_raw = match &self.logic {
                ControllerLogic::EffectiveAngleOfAttack(controller) => {
                    controller.get_new_output(&input.angles_of_attack)
                },
                ControllerLogic::WeatherDependentSetPoints(controller) => {
                    controller.get_new_output(&input.apparent_wind_directions)
                }
            };

            return Some(new_output_raw)
        }

        None
    }
}


