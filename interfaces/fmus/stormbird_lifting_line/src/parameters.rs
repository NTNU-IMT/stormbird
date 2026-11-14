// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Deserialize, Serialize};

use std::path::Path;

use stormbird::error::Error;
use stormbird::lifting_line::complete_sail_model::builder::CompleteSailModelBuilder;

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
    pub number_of_iterations_before_building_model: usize
}

impl FmuParameters {
    /// Construct a new Parameters object from a JSON file.
    pub fn from_json_file(file_path: &Path) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;

        Ok(serde_json::from_str(&json_string)?)
    }
}
