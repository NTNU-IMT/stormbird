// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use crate::lifting_line::simulation_builder::SimulationBuilder;
use crate::wind::environment::WindEnvironment;
use crate::controller::builder::ControllerBuilder;
//use crate::empirical_models::input_power::InputPower;

use super::CompleteSailModel;

use crate::error::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteSailModelBuilder {
    lifting_line_simulation: SimulationBuilder,
    wind_environment: WindEnvironment,
    controller: ControllerBuilder,
}

impl CompleteSailModelBuilder {
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let builder = serde_json::from_str(setup_string)?;
        
        Ok(builder)
    }

    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let string = std::fs::read_to_string(file_path)?;

        Self::new_from_string(&string)
    }

    pub fn build(&self) -> CompleteSailModel {
        CompleteSailModel {
            lifting_line_simulation: self.lifting_line_simulation.build(),
            wind_environment: self.wind_environment.clone(),
            controller: self.controller.build()
        }
    }
}
