// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Lifting line solvers

use serde::{Deserialize, Serialize};

//pub mod quasi_newton;
pub mod iterative;
/// The linearized lifting line solver
pub mod linearized;
pub mod velocity_corrections;

use iterative::{
    Iterative,
    QuasiSteadyIterativeBuilder
};
use linearized::Linearized;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The solver used to find the circulation strength in a lifting line simulation
pub enum Solver {
    /// Iterative solver
    Iterative(Iterative),
    /// Linearized solver
    Linearized(Linearized)
}

impl Default for Solver {
    fn default() -> Self {
        Solver::Iterative(Iterative::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Builder for the solver in quasi-steady simulations
pub enum QuasiSteadySolverBuilder {
    /// Iterative solver
    Iterative(QuasiSteadyIterativeBuilder),
    /// Linearized solver
    Linearized(Linearized)
}

impl Default for QuasiSteadySolverBuilder {
    fn default() -> Self {
        QuasiSteadySolverBuilder::Linearized(Linearized::default())
    }
}

impl QuasiSteadySolverBuilder {
    /// Builds the solver
    pub fn build(&self) -> Solver {
        match self {
            Self::Linearized(settings) => {
                Solver::Linearized(settings.clone())
            },
            Self::Iterative(builder) => {
                Solver::Iterative(builder.build())
            }
        }
    }

}

/// Re-exports of commonly used structures, to simplify imports
pub mod prelude {
    pub use super::Solver;
    pub use super::QuasiSteadySolverBuilder;
}

