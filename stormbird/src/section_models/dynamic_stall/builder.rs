// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Builder for the [dynamic stall model](DynamicStallModel).

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use super::{DynamicStallModel, DynamicStallSegmentState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for a [dynamic stall model](DynamicStallModel) on a single wing. Only contains the
/// parameters of the model. The internal state of the model, i.e., the memory effects for each line
/// segment, is created when the model is built, as the number of line segments is then known.
pub struct DynamicStallModelBuilder {
    #[serde(default = "DynamicStallModelBuilder::default_pressure_time_constant")]
    /// Non-dimensional time constant for the lag in the pressure distribution, T_p, given in
    /// semi-chords travelled. A value of zero turns the lag off.
    pub pressure_time_constant: Float,
    #[serde(default = "DynamicStallModelBuilder::default_separation_time_constant")]
    /// Non-dimensional time constant for the lag in the boundary layer separation, T_f, given in
    /// semi-chords travelled. A value of zero turns the lag off.
    pub separation_time_constant: Float,
}

impl DynamicStallModelBuilder {
    /// Default value for the pressure time constant, typical for a NACA 0012 at low Mach numbers
    pub fn default_pressure_time_constant() -> Float {1.7}
    /// Default value for the separation time constant, typical for a NACA 0012 at low Mach numbers
    pub fn default_separation_time_constant() -> Float {3.0}

    /// Builds the dynamic stall model for a wing with the given number of line segments.
    ///
    /// # Panics
    /// If any of the time constants are negative.
    pub fn build(&self, nr_segments: usize) -> DynamicStallModel {
        if self.pressure_time_constant < 0.0 || self.separation_time_constant < 0.0 {
            panic!(
                "The time constants in the dynamic stall model can not be negative. \
                pressure_time_constant = {}, separation_time_constant = {}",
                self.pressure_time_constant,
                self.separation_time_constant
            );
        }

        DynamicStallModel {
            pressure_time_constant: self.pressure_time_constant,
            separation_time_constant: self.separation_time_constant,
            time_step: 0.0,
            segment_states: vec![DynamicStallSegmentState::default(); nr_segments],
            is_initialized: false,
        }
    }
}

impl Default for DynamicStallModelBuilder {
    fn default() -> Self {
        Self {
            pressure_time_constant: Self::default_pressure_time_constant(),
            separation_time_constant: Self::default_separation_time_constant(),
        }
    }
}
