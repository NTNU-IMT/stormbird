// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A builder for the [Simulation] struct.

use serde::{Serialize, Deserialize};

use crate::lifting_line::prelude::*;

use super::simulation::Simulation;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a quasi-steady simulation.
pub struct SteadySettings {
    #[serde(default)]
    pub solver: SteadySolverSettings,
    #[serde(default)]
    pub wake: WakeBuilder,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a dynamic simulation.
pub struct UnsteadySettings {
    #[serde(default)]
    pub solver: UnsteadySolverSettings,
    #[serde(default)]
    pub wake: WakeBuilder,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
/// Enum that determines what type of simulation to run. 
/// 
/// Both quasi-steady and dynamic simulations are supported. The settings for each simulation type 
/// is stored as a member variable of each variant.
pub enum SimulationMode {
    QuasiSteady(SteadySettings),
    Dynamic(UnsteadySettings),
}

impl Default for SimulationMode {
    fn default() -> Self {
        SimulationMode::QuasiSteady(SteadySettings::default())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
/// Builder for the [Simulation] struct.
pub struct SimulationBuilder {
    pub line_force_model: LineForceModelBuilder,
    #[serde(default)]
    pub simulation_mode: SimulationMode,
    #[serde(default)]
    pub write_wake_data_to_file: bool,
    #[serde(default)]
    pub wake_files_folder_path: String,
}

impl SimulationBuilder {
    pub fn new(
        line_force_model: LineForceModelBuilder,
        simulation_mode: SimulationMode,
    ) -> Self {
        SimulationBuilder {
            line_force_model,
            simulation_mode,
            write_wake_data_to_file: false,
            wake_files_folder_path: String::new()
        }
    }

    /// Creates a new simulation builder by parsing the the string as a JSON object. The parsing is
    /// done using the serde_json library.
    pub fn new_from_string(string: &str) -> Result<Self, String> {
        match serde_json::from_str(string) {
            Ok(builder) => Ok(builder),
            Err(e) => Err(format!("Error parsing JSON: {}", e))
        }
    }

    /// Creates a new simulation builder by reading the file at the given path and parsing the 
    /// content as a JSON object. The parsing is done using the [SimulationBuilder::new_from_string]
    /// method.
    pub fn new_from_file(file_path: &str) -> Result<Self, String> {
        match std::fs::read_to_string(file_path) {
            Ok(string) => Self::new_from_string(&string),
            Err(e) => Err(format!("Error reading file: {}", e))
        }
    }

    /// Builds the [Simulation] struct based on the current state of the builder.
    pub fn build(&self, initial_time_step: f64, wake_initial_velocity: SpatialVector<3>) -> Simulation {
        let line_force_model = self.line_force_model.build();
        let nr_of_lines = line_force_model.nr_span_lines();

        let wake = match &self.simulation_mode {
            SimulationMode::Dynamic(settings) => {
                settings.wake.build(
                    initial_time_step,
                    &line_force_model,
                    wake_initial_velocity,
                )
            },
            SimulationMode::QuasiSteady(settings) => {
                settings.wake.build(
                    initial_time_step,
                    &line_force_model,
                    wake_initial_velocity,
                )
            }
        };

        let solver_settings = match &self.simulation_mode {
            SimulationMode::Dynamic(settings) => {
                settings.solver.to_solver_settings()
            },
            SimulationMode::QuasiSteady(settings) => {
                settings.solver.to_solver_settings()
            }
        };

        Simulation {
            line_force_model,
            wake,
            solver_settings,
            derivatives: None,
            previous_circulation_strength: vec![0.0; nr_of_lines],
            write_wake_data_to_file: self.write_wake_data_to_file,
            wake_files_folder_path: self.wake_files_folder_path.clone()
        }
    }
}