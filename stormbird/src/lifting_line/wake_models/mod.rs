// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of wake models used to calculate induced velocities in lifting line simulations

pub mod steady;
pub mod unsteady;
pub mod velocity_corrections;

pub mod frozen_wake;

use math_utils::spatial_vector::SpatialVector;
use crate::line_force_model::LineForceModel;

use steady::{SteadyWakeBuilder, SteadyWake};
use unsteady::UnsteadyWake;

#[derive(Debug, Clone)]
/// Enum to stor different types of wake models.
pub enum WakeModel {
    Steady((SteadyWakeBuilder, SteadyWake)),
    Unsteady(UnsteadyWake)
}

impl WakeModel {
    pub fn pre_solver_initialization(
        &mut self, 
        line_force_model: &LineForceModel, 
        ctrl_points_freestream: &[SpatialVector<3>]
    ) {
        match self {
            WakeModel::Steady((builder, wake)) => {
                *wake = builder.build(
                    line_force_model,
                    ctrl_points_freestream
                );
            },
            WakeModel::Unsteady(wake) => {
                wake.synchronize_wing_geometry(line_force_model);
            }
        }
    }
    
}

/// Typical imports when using the velocity models
pub mod prelude {
    pub use super::WakeModel;
    pub use super::steady::{SteadyWakeBuilder, SteadyWake};
    pub use super::unsteady::{UnsteadyWakeBuilder, UnsteadyWake};
}

#[cfg(test)]
mod tests;