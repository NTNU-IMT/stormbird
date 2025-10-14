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
    pub input_ctrl_point_velocity: Vec<SpatialVector>,
    pub circulation_strength: Vec<Float>,
    pub output_ctrl_point_velocity: Vec<SpatialVector>,
    pub iterations: usize,
    pub residual: Float,
}