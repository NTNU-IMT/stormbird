// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Lifting line solvers

//pub mod quasi_newton;
pub mod simple_iterative;
pub mod velocity_corrections;

pub mod prelude {
    pub use super::simple_iterative::{
        SimpleIterative,
        SteadySimpleIterativeBuilder,
    };
}