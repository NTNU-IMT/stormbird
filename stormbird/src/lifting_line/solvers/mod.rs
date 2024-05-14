// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Lifting line solvers

use crate::vec3::Vec3;
use crate::line_force_model::prelude::*;
use crate::io_structs::prelude::*;

pub mod steady;
pub mod unsteady;

#[derive(Debug, Clone)]
pub struct SolverResult {
    pub circulation_strength: Vec<f64>,
    pub ctrl_point_velocity: Vec<Vec3>,
}

/// Returns the **felt velocity** at the control points, neglecting the lift-induced velocities. 
/// That is, it combines the freestream velocity with the felt velocity due to the motion of the
/// wing.
pub fn calculate_felt_ctrl_points_freestream(
    ctrl_points_freestream: &[Vec3],
    time_step: f64,
    derivatives: Option<&Derivatives>,
    line_force_model: &LineForceModel,
) -> Vec<Vec3> {
    let mut ctrl_point_velocity = Vec::with_capacity(ctrl_points_freestream.len());

    let motion_velocity = if let Some(derivatives) = derivatives {
        derivatives.motion.ctrl_point_velocity(line_force_model, time_step)
    } else {
        vec![Vec3::default(); line_force_model.nr_span_lines()]
    };

    for i in 0..ctrl_points_freestream.len() {
        ctrl_point_velocity.push(
            ctrl_points_freestream[i] - motion_velocity[i]
        );
    }

    ctrl_point_velocity
}

pub mod solver_settings {
    pub use super::steady::SteadySolverSettings;
    pub use super::unsteady::UnsteadySolverSettings;
}