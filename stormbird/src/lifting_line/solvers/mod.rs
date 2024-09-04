// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Lifting line solvers

use serde::{Deserialize, Serialize};

use math_utils::spatial_vector::SpatialVector;
use crate::line_force_model::prelude::*;
use crate::lifting_line::wake::prelude::*;

pub mod quasi_newton;
pub mod simple_iterative;
pub mod velocity_corrections;

use quasi_newton::{
    QuasiNewtonBuilder,
    QuasiNewton,
};

use simple_iterative::{
    SimpleIterative,
    SteadySimpleIterativeBuilder,
};

use super::prelude::SolverResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolverBuilder {
    SimpleIterative(SimpleIterative),
    QuasiNewton(QuasiNewtonBuilder),
}

impl Default for SolverBuilder {
    fn default() -> Self {
        SolverBuilder::SimpleIterative(SimpleIterative::default())
    }
}

impl SolverBuilder {
    pub fn build(&self, nr_span_lines: usize) -> Solver {
        match self {
            SolverBuilder::SimpleIterative(s) => Solver::SimpleIterative(
                s.clone()
            ),
            SolverBuilder::QuasiNewton(qn) => Solver::QuasiNewton(
                qn.build(nr_span_lines)
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SteadySolverBuilder {
    SimpleIterative(SteadySimpleIterativeBuilder),
    QuasiNewton(QuasiNewtonBuilder),
}

impl Default for SteadySolverBuilder {
    fn default() -> Self {
        SteadySolverBuilder::SimpleIterative(SteadySimpleIterativeBuilder::default())
    }
}

impl SteadySolverBuilder {
    pub fn build(&self, nr_span_lines: usize) -> Solver {
        match self {
            SteadySolverBuilder::SimpleIterative(solver_builder) => Solver::SimpleIterative(
                solver_builder.build()
            ),
            SteadySolverBuilder::QuasiNewton(solver_builder) => Solver::QuasiNewton(
                solver_builder.build(nr_span_lines)
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Solver {
    SimpleIterative(SimpleIterative),
    QuasiNewton(QuasiNewton),
}

impl Solver {
    pub fn do_step(
        &mut self, 
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector<3>],
        frozen_wake: &FrozenWake,
        initial_solution: &[f64],
    ) -> SolverResult {
        match self {
            Solver::SimpleIterative(solver) => solver.do_step(
                line_force_model,
                felt_ctrl_points_freestream,
                frozen_wake,
                initial_solution,
            ),
            Solver::QuasiNewton(solver) => solver.do_step(
                line_force_model,
                felt_ctrl_points_freestream,
                frozen_wake,
                initial_solution,
            ),
        }
    }
}

pub mod prelude {
    pub use super::Solver;
    pub use super::{
        SolverBuilder,
        SteadySolverBuilder,
    };
    pub use super::quasi_newton::QuasiNewtonBuilder;
    pub use super::simple_iterative::{
        SimpleIterative,
        SteadySimpleIterativeBuilder,
    };
}