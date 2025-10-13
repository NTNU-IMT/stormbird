// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of lifting line functionality. 
//! 
//! This mainly consist of functions that can solve for vortex strength and induced velocity on a 
//! wing based on potential theory and line representation of wings. Uses the 
//! [crate::line_force_model] as input

pub mod wake;
pub mod solvers;
pub mod singularity_elements;
pub mod simulation_builder;
pub mod simulation;
pub mod complete_sail_model;

/// Typical imports when using the lifting line functionality
pub mod prelude {
    pub use stormath::spatial_vector::SpatialVector;
    pub use crate::common_utils::prelude::*;
    pub use crate::line_force_model::prelude::*;

    pub use super::wake::prelude::*;

    pub use super::singularity_elements::prelude::*;

    pub use super::solvers::prelude::*;

    pub use crate::section_models::{
        SectionModel,
        foil::Foil,
        rotating_cylinder::RotatingCylinder,
    };
}

#[cfg(test)]
mod tests;