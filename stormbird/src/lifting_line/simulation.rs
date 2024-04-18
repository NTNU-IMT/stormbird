// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! An interface for running simulations using the lifting line models.
//! 
//! To support different use cases, both quasi-steady and true dynamic simulations are supported. 
//! However, the interface is designed to be as unified as possible. 

use serde::{Serialize, Deserialize};

use crate::lifting_line::prelude::*;
use crate::line_force_model::velocity_input::motion::MotionCalculator;
use crate::line_force_model::velocity_input::InputState;


#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a quasi-steady simulation.
pub struct SteadySettings {
    #[serde(default)]
    pub solver: SteadySolverSettings,
    #[serde(default)]
    pub wake: SteadyWakeBuilder,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a dynamic simulation.
pub struct UnsteadySettings {
    #[serde(default)]
    pub solver: UnsteadySolverSettings,
    #[serde(default)]
    pub wake: UnsteadyWakeBuilder,
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
    pub fn build(&self) -> Simulation {
        let line_force_model = self.line_force_model.build();
        let nr_of_lines = line_force_model.nr_span_lines();

        Simulation {
            line_force_model,
            simulation_mode: self.simulation_mode.clone(),
            motion_calculator: None,
            previous_circulation_strength: vec![0.0; nr_of_lines],
            unsteady_wake: None,
            write_wake_data_to_file: self.write_wake_data_to_file,
            wake_files_folder_path: self.wake_files_folder_path.clone()
        }
    }
}

#[derive(Debug, Clone)]
/// Struct that contains the data needed to run a dynamic simulation.
pub struct Simulation {
    pub line_force_model: LineForceModel,
    simulation_mode: SimulationMode,
    previous_circulation_strength: Vec<f64>,
    motion_calculator: Option<MotionCalculator>,
    unsteady_wake: Option<UnsteadyWake>,
    write_wake_data_to_file: bool,
    wake_files_folder_path: String,
}

impl Simulation {
    pub fn new_from_string(string: &str) -> Result<Self, String> {
        let builder = SimulationBuilder::new_from_string(string)?;

        Ok(builder.build())
    }

    pub fn do_step(
        &mut self, 
        time: f64,
        time_step: f64,
        input_state: InputState
    ) -> SimulationResult {
        self.line_force_model.rotation    = input_state.rotation;
        self.line_force_model.translation = input_state.translation;

        // If the force input calculator has not been initialized, initialize it.
        if self.motion_calculator.is_none() {
            self.motion_calculator = Some(MotionCalculator::new(&self.line_force_model));
        }

        let motion_calculator = self.motion_calculator.as_mut().unwrap();

        let motion = motion_calculator.get_motion(
            &self.line_force_model,
            time_step
        );

        let result = match &self.simulation_mode {
            SimulationMode::QuasiSteady(settings) => {
                steady_solvers::solve_steady(
                    &self.line_force_model, 
                    &input_state.freestream,
                    &motion,
                    &settings.solver,
                    &settings.wake, 
                    &self.previous_circulation_strength
                )
            },
            SimulationMode::Dynamic(settings) => {
                if self.unsteady_wake.is_none() {
                    self.unsteady_wake = Some(
                        settings.wake.build(
                            time_step,
                            &self.line_force_model,
                            &input_state.freestream
                        )
                    );
                }

                let mut wake = self.unsteady_wake.as_mut().unwrap();

                let result = unsteady_solvers::solve_one_time_step(
                    time_step,
                    &self.line_force_model,
                    &input_state.freestream,
                    &motion,
                    &mut wake,
                    &settings.solver,
                    &self.previous_circulation_strength
                );

                let time_step_index = (time / time_step) as usize;

                if self.write_wake_data_to_file {
                    let wake_file_path = format!("{}/wake_{}.vtp", self.wake_files_folder_path, time_step_index);

                    let write_result = wake.write_wake_to_vtk_file(&wake_file_path);

                    match write_result {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Error writing wake data to file: {}", e);
                        }
                    }
                }

                result
            }
        };

        self.previous_circulation_strength = result.circulation_strength.clone();

        result
    }
}