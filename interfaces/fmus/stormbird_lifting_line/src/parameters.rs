// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Deserialize, Serialize};

use std::path::Path;

use stormbird::error::Error;
use stormbird::lifting_line::complete_sail_model::builder::CompleteSailModelBuilder;
use stormbird::lifting_line::simulation_builder::SimulationSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parameters for the Stormbird lifting line FMU, read from a single JSON file. These variables
/// could also be part of the FMU directly. However, they are stored in a separate JSON file to
/// facilitate easier usage when using the FMU with runtimes that have limited to no support for
/// other data types than f64.
pub struct FmuParameters {
    /// The complete sail model, i.e., the lifting line simulation, the wind environment and the
    /// optional controller.
    pub sail_model: CompleteSailModelBuilder,
    #[serde(default)]
    /// Switch to specify if angles is given in degrees or radians.
    pub angles_in_degrees: bool,
    #[serde(default)]
    /// Switch to specify whether or not to use the motion velocity input
    pub use_motion_velocity: bool,
    #[serde(default)]
    /// Switch to specify which coordinate system the input motion velocity is given in.
    pub motion_velocity_in_body_fixed_frame: bool,
    #[serde(default)]
    /// Switch to specify whether to apply the motion velocity directly or to use the linear
    /// velocity as freestream. WARNING: this switch must be used with care. It does not make sense
    /// to set this to true if the translation is also actively used. This should only be used if
    /// the translation is always zero. It can be used with rotation.
    pub use_motion_velocity_linear_as_freestream: bool,
    #[serde(default)]
    /// If larger than zero, this variable is used to construct moving average filters on the input
    pub input_moving_average_window_size: usize,
    #[serde(default)]
    /// If larger than zero, the model is not simulated until this number of time steps have been
    /// completed. This is useful in situations where the input velocity may not be properly set
    /// until a couple of time steps into the simulation.
    pub number_of_iterations_before_building_model: usize,
    #[serde(default)]
    /// If true, each time step is a steady state simulation with the steady state methods of the
    /// complete sail model, rather than a step in a time-domain simulation. The sails are placed at
    /// the input translation and rotation without any motion velocity, the ship velocity is the
    /// linear motion velocity input, and the controller, if any, is iterated until it converges.
    /// The lifting line simulation must use quasi-steady settings.
    pub steady_state: bool,
    #[serde(default)]
    /// If true, the steady state simulations search for the controller loading that gives the
    /// highest effective power, up to the `controller_loading` input. Only used together with
    /// `steady_state`.
    pub steady_state_optimize_loading: bool,
}

impl FmuParameters {
    /// Checks that the settings are consistent
    pub fn check(&self) -> Result<(), Error> {
        if self.steady_state_optimize_loading && !self.steady_state {
            return Err(Error::invalid_input(
                "steady_state_optimize_loading can only be used together with steady_state"
            ));
        }

        let is_quasi_steady = matches!(
            self.sail_model.lifting_line_simulation.simulation_settings,
            SimulationSettings::QuasiSteady(_)
        );

        if self.steady_state && !is_quasi_steady {
            return Err(Error::invalid_input(
                "The steady state mode of the FMU requires quasi-steady settings for the lifting \
                line simulation"
            ));
        }

        Ok(())
    }

    /// Construct a new Parameters object from a JSON file.
    pub fn from_json_file(file_path: &Path) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;

        Ok(serde_json::from_str(&json_string)?)
    }
}
