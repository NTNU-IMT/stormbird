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
    /// Builder for the solver
    #[serde(default)]
    pub solver: QuasiSteadySolverBuilder,
    /// Settings for the quasi-steady wake
    #[serde(default)]
    pub wake: QuasiSteadyWakeSettings,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a dynamic simulation.
pub struct DynamicSettings {
    /// The solver
    #[serde(default)]
    pub solver: Solver,
    /// Builder for the dynamic wake
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
    /// Quasi-steady simulation, where the wake is made of horseshoe vortices aligned with the
    /// freestream in each time step
    QuasiSteady(QuasiSteadySettings),
    /// Dynamic simulation, where the wake is made of vortex panels that are shed from the wings and
    /// moved with the flow
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
    /// Builder for the line force model
    pub line_force_model: LineForceModelBuilder,
    /// Settings that determine the type of simulation
    #[serde(default)]
    pub simulation_settings: SimulationSettings,
}

impl SimulationBuilder {
    /// Creates a new builder from a line force model builder and simulation settings
    pub fn new(
        line_force_model: LineForceModelBuilder,
        simulation_settings: SimulationSettings,
    ) -> Self {
        SimulationBuilder {
            line_force_model,
            simulation_settings,
        }
    }

    /// Creates a new simulation builder by parsing the string as a JSON object. The parsing is
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

    /// Checks that the settings are consistent. At the moment, this only checks that dynamic stall
    /// models are only used together with a dynamic simulation.
    pub fn check_settings(&self) -> Result<(), Error> {
        let has_dynamic_stall = self.line_force_model.wing_builders.iter()
            .any(|wing_builder| wing_builder.dynamic_stall.is_some());

        if has_dynamic_stall && matches!(self.simulation_settings, SimulationSettings::QuasiSteady(_)) {
            return Err(Error::CustomStringError(
                "The dynamic stall model can only be used together with dynamic simulation \
                settings, as it depends on the time history of the flow".to_string()
            ));
        }

        Ok(())
    }

    /// Builds the [Simulation] struct based on the current state of the builder.
    ///
    /// # Panics
    /// If the settings are not consistent, see [SimulationBuilder::check_settings].
    pub fn build(&self) -> Simulation {
        if let Err(error) = self.check_settings() {
            panic!("Invalid simulation settings: {}", error);
        }

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
                    QuasiSteadyWake::new(settings.wake.clone())
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