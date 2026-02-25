// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html) 

use crate::error::Error;

use serde::{Deserialize, Serialize};

use super::Controller;
use super::set_points::ControllerSetPoints;
use super::measurements::FlowMeasurementSettings;

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerBuilder {
    pub set_points: Vec<ControllerSetPoints>,
    #[serde(default)]
    pub flow_measurement_settings: FlowMeasurementSettings,
    #[serde(default = "ControllerBuilder::default_time_steps_between_updates")]
    pub time_steps_between_updates: usize,
    #[serde(default)]
    pub start_time: Float,
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
            set_points: self.set_points.clone(),
            flow_measurement_settings: self.flow_measurement_settings.clone(),
            time_steps_between_updates: self.time_steps_between_updates,
            start_time: self.start_time,
            time_step_index: 0,
            use_input_velocity_for_apparent_wind_direction: self.use_input_velocity_for_apparent_wind_direction,
        }
    }
}
