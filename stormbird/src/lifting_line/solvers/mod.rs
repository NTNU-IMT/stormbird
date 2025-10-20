// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Lifting line solvers

use serde::{Deserialize, Serialize};

//pub mod quasi_newton;
pub mod simple_iterative;
pub mod linearized;
pub mod velocity_corrections;

use simple_iterative::{
    SimpleIterative,
    QuasiSteadySimpleIterativeBuilder
};
use linearized::Linearized;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Solver {
    SimpleIterative(SimpleIterative),
    Linearized(Linearized)
}

impl Default for Solver {
    fn default() -> Self {
        Solver::SimpleIterative(SimpleIterative::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuasiSteadySolverBuilder {
    SimpleIterative(QuasiSteadySimpleIterativeBuilder),
    Linearized(Linearized)
}

impl Default for QuasiSteadySolverBuilder {
    fn default() -> Self {
        QuasiSteadySolverBuilder::Linearized(Linearized::default())
    }
}

impl QuasiSteadySolverBuilder {
    pub fn build(&self) -> Solver {
        match self {
            Self::Linearized(settings) => {
                Solver::Linearized(settings.clone())
            },
            Self::SimpleIterative(builder) => {
                Solver::SimpleIterative(builder.build())
            }
        }
    }

}

pub mod prelude {
    pub use super::Solver;
    pub use super::QuasiSteadySolverBuilder;
}

