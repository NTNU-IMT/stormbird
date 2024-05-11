// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementaiton of lifting line functionality. 
//! 
//! This mainly consist of functions that can solve for vortex strength and induced velocity on a 
//! wing based on potential theory and line representation of wings. Uses the 
//! [crate::line_force_model] as input

pub mod wake_models;
pub mod solvers;
pub mod singularity_elements;
pub mod simulation;
pub mod velocity_input;

/// Typical imports when using the lifting line functionality
pub mod prelude {
    pub use crate::vec3::Vec3;
    pub use crate::io_structs::prelude::*;

    pub use super::wake_models::prelude::*;

    pub use super::singularity_elements::prelude::*;

    pub use super::solvers::steady as steady_solvers;
    pub use super::solvers::unsteady as unsteady_solvers;
    pub use super::solvers::solver_settings::*;
    
    pub use crate::line_force_model::span_line::SpanLine;
    pub use crate::line_force_model::LineForceModel;
    pub use crate::line_force_model::builder::{
        LineForceModelBuilder,
        WingBuilder,
    };

    pub use crate::section_models::{
        SectionModel,
        foil::Foil,
        rotating_cylinder::RotatingCylinder,
    };
    pub use crate::empirical_models::viscous_wakes::ViscousWakes;
}

#[cfg(test)]
mod tests;