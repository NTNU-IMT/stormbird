

use serde::{Serialize, Deserialize};

use crate::lifting_line::simulation_builder::SimulationBuilder;
use crate::wind::environment::WindEnvironment;
use crate::controllers::ControllerBuilder;

use super::CompleteSailModel;

use crate::error::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompleteSailModelBuilder {
    lifting_line_simulation: SimulationBuilder,
    wind_environment: WindEnvironment,
    controller: ControllerBuilder
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
        let lifting_line_simulation = self.lifting_line_simulation.build();

        let controller = self.controller.build();

        CompleteSailModel {
            lifting_line_simulation,
            wind_environment: self.wind_environment.clone(),
            controller
        }
    }
}