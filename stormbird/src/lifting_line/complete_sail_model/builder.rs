// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use crate::lifting_line::simulation_builder::SimulationBuilder;
use crate::wind::environment::WindEnvironment;
use crate::controller::builder::ControllerBuilder;
use crate::controller::measurements::SpanwiseMeasurementBuilder;

use super::CompleteSailModel;
use super::settings::CompleteSailModelSettings;

use crate::error::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the complete sail model
pub struct CompleteSailModelBuilder {
    /// Builder for the lifting line simulation of the sails
    pub lifting_line_simulation: SimulationBuilder,
    /// The model of the wind environment. The default wind environment is used if not given.
    #[serde(default)]
    pub wind_environment: WindEnvironment,
    /// Optional builder for the control system of the sails. Without a controller, the sails are
    /// operated with the local wing angles and internal states that are set directly on the model.
    #[serde(default)]
    pub controller: Option<ControllerBuilder>,
    /// Settings that are specific to the complete sail model
    #[serde(default)]
    pub settings: CompleteSailModelSettings
}

impl CompleteSailModelBuilder {
    /// Creates a builder from a JSON string
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let builder = serde_json::from_str(setup_string)?;
        
        Ok(builder)
    }

    /// Creates a builder from a JSON file
    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let string = std::fs::read_to_string(file_path)?;

        Self::new_from_string(&string)
    }

    /// Builds the complete sail model
    pub fn build(&self) -> Result<CompleteSailModel, Error> {
        let lifting_line_simulation = self.lifting_line_simulation.build()?;

        let line_force_model = &lifting_line_simulation.line_force_model;

        let controller = match &self.controller {
            Some(controller_builder) => Some(controller_builder.build(line_force_model)?),
            None => None,
        };

        let default_spanwise_measurement = SpanwiseMeasurementBuilder::default()
            .build(line_force_model)?;

        Ok(CompleteSailModel {
            lifting_line_simulation,
            wind_environment: self.wind_environment.clone(),
            controller,
            settings: self.settings.clone(),
            default_spanwise_measurement,
        })
    }
}
