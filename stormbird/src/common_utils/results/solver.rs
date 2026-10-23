// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::{
    spatial_vector::SpatialVector,
    type_aliases::Float,
};

#[derive(Debug, Clone)]
/// Results from a lifting line solver, which will be further used to generate SimulationResults
pub struct SolverResult {
    /// The velocity at the control points used as input to the solver
    pub input_ctrl_points_velocity: Vec<SpatialVector>,
    /// The circulation strength on each line element found by the solver
    pub circulation_strength: Vec<Float>,
    /// The velocity at the control points from the final solution, including induced velocities
    pub output_ctrl_points_velocity: Vec<SpatialVector>,
    /// The number of iterations used by the solver
    pub iterations: usize,
    /// The residual of the final solution
    pub residual: Float,
}