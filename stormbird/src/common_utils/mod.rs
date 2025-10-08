// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Collection of structs used for multiple simulations methods, i.e., useful for both lifting line
//! and actuator line methods.

pub mod results;
pub mod forces_and_moments;
pub mod flow_derivatives;

pub mod prelude {
    pub use super::results::solver::SolverResult;
    pub use super::results::simulation::SimulationResult;
    pub use super::forces_and_moments::*;
    pub use super::flow_derivatives::*;
}