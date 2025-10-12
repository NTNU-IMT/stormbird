// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A builder for the [Simulation] struct.

use serde::{Serialize, Deserialize};

use crate::lifting_line::prelude::*;
use crate::lifting_line::wake::{
    frozen_wake::FrozenWake,
    dynamic_wake::builder::DynamicWakeBuilder,
};

use crate::lifting_line::wake::settings::QuasiSteadyWakeSettings;

use super::simulation::Simulation;

use crate::error::Error;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a quasi-steady simulation.
pub struct QuasiSteadySettings {
    #[serde(default)]
    pub solver: QuasiSteadySolverBuilder,
    #[serde(default)]
    pub wake: QuasiSteadyWakeSettings,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a dynamic simulation.
pub struct DynamicSettings {
    #[serde(default)]
    pub solver: Solver,
    #[serde(default)]
    pub wake: DynamicWakeBuilder,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
/// Enum that determines what type of simulation to run. 
/// 
/// Both quasi-steady and dynamic simulations are supported. The settings for each simulation type 
/// is stored as a member variable of each variant.
pub enum SimulationSettings {
    QuasiSteady(QuasiSteadySettings),
    Dynamic(DynamicSettings),
}

impl Default for SimulationSettings {
    fn default() -> Self {
        SimulationSettings::QuasiSteady(QuasiSteadySettings::default())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
/// Builder for the [Simulation] struct.
pub struct SimulationBuilder {
    pub line_force_model: LineForceModelBuilder,
    #[serde(default)]
    pub simulation_settings: SimulationSettings,
}

impl SimulationBuilder {
    pub fn new(
        line_force_model: LineForceModelBuilder,
        simulation_settings: SimulationSettings,
    ) -> Self {
        SimulationBuilder {
            line_force_model,
            simulation_settings,
        }
    }

    /// Creates a new simulation builder by parsing the the string as a JSON object. The parsing is
    /// done using the serde_json library.
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let builder = serde_json::from_str(setup_string)?;
        
        Ok(builder)
    }

    /// Creates a new simulation builder by reading the file at the given path and parsing the 
    /// content as a JSON object. The parsing is done using the [SimulationBuilder::new_from_string]
    /// method.
    pub fn new_from_file(file_path: &str) -> Result<Self, Error> {
        let string = std::fs::read_to_string(file_path)?;

        Self::new_from_string(&string)
    }

    /// Builds the [Simulation] struct based on the current state of the builder.
    pub fn build(&self) -> Simulation {
        let line_force_model = self.line_force_model.build();

        let nr_of_lines = line_force_model.nr_span_lines();

        let wake_data = match &self.simulation_settings {
            SimulationSettings::Dynamic(settings) => {
                WakeData::Dynamic(
                    settings.wake.build(
                        &line_force_model
                    )
                )
            },
            SimulationSettings::QuasiSteady(settings) => {
                WakeData::QuasiSteady(
                    settings.wake.clone()
                )
            }
        };

        let frozen_wake = FrozenWake::initialize(nr_of_lines);

        let solver = match &self.simulation_settings {
            SimulationSettings::Dynamic(settings) => {
                settings.solver.clone()
            },
            SimulationSettings::QuasiSteady(settings) => {
                settings.solver.build()
            }
        };

        let previous_circulation_strength = vec![0.0; nr_of_lines];

        let flow_derivatives = FlowDerivatives::new(
            &vec![SpatialVector::from([0.0, 0.0, 0.0]); nr_of_lines]
        );

        Simulation {
            line_force_model,
            flow_derivatives,
            wake_data,
            frozen_wake,
            solver,
            previous_circulation_strength,
            first_time_step_completed: false,
        }
    }
}