// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of various controllers for wind propulsion devices intended to be used together
//! with the simulation models in the library. 

pub mod input;
pub mod output;
pub mod measurements;
pub mod logic;
pub mod prelude;

use crate::error::Error;

use serde::{Deserialize, Serialize};

use input::ControllerInput;
use output::ControllerOutput;
use logic::ControllerLogic;
use measurements::FlowMeasurementSettings;

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerBuilder {
    pub logic: ControllerLogic,
    #[serde(default)]
    pub flow_measurement_settings: FlowMeasurementSettings,
    #[serde(default = "ControllerBuilder::default_time_steps_between_updates")]
    pub time_steps_between_updates: usize,
    #[serde(default)]
    pub start_time: Float,
    #[serde(default)]
    pub max_local_wing_angle_change_rate: Option<Float>,
    #[serde(default)]
    pub max_internal_section_state_change_rate: Option<Float>,
    #[serde(default)]
    pub moving_average_window_size: Option<usize>,
    #[serde(default)]
    pub use_input_velocity_for_apparent_wind_direction: bool,
}

impl ControllerBuilder {
    pub fn default_time_steps_between_updates() -> usize {1}

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
            time_step_index: 0,
            use_input_velocity_for_apparent_wind_direction: self.use_input_velocity_for_apparent_wind_direction,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Controller {
    pub logic: ControllerLogic,
    pub flow_measurement_settings: FlowMeasurementSettings,
    pub time_steps_between_updates: usize,
    pub start_time: Float,
    pub max_local_wing_angle_change_rate: Option<Float>,
    pub max_internal_section_state_change_rate: Option<Float>,
    pub time_step_index: usize,
    pub use_input_velocity_for_apparent_wind_direction: bool,
}

impl Controller {
    pub fn update(
        &self,
        time: Float,
        time_step: Float, 
        input: &ControllerInput,
    ) -> Option<ControllerOutput> {
        let initialization_done = time >= self.start_time;
        let time_to_update =  self.time_step_index % self.time_steps_between_updates == 0;
        let first_time_step = self.time_step_index == 1;

        if first_time_step || (time_to_update && initialization_done) {
            let mut output = self.logic.get_new_output(input);

            if self.max_local_wing_angle_change_rate.is_some() && output.local_wing_angles.is_some() {
                let raw_new_values = output.local_wing_angles.as_ref().unwrap();
                let max_change = self.max_local_wing_angle_change_rate.unwrap() * time_step;

                output.local_wing_angles = Some(
                    output::limit_values(
                        &input.current_local_wing_angles, 
                        raw_new_values, 
                        max_change
                    )
                )
            }

            if self.max_local_wing_angle_change_rate.is_some() && output.section_models_internal_state.is_some() {
                let raw_new_values = output.section_models_internal_state.as_ref().unwrap();
                let max_change = self.max_internal_section_state_change_rate.unwrap() * time_step;

                output.section_models_internal_state = Some(
                    output::limit_values(
                        &input.current_section_models_internal_state, 
                        raw_new_values, 
                        max_change
                    )
                )
            }

            return Some(output)
        }

        None
    }
}


