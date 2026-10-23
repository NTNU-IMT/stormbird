// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of the necessary singularity elements for the lifting line method. 

use symmetry_condition::SymmetryCondition;

pub mod vortex_line;
/// Symmetry planes for the induced velocity calculations
pub mod symmetry_condition;
/// Quadrilateral vortex panels
pub mod panel;
/// Horseshoe vortices
pub mod horseshoe_vortex;

use stormath::type_aliases::Float;

#[derive(Clone, Debug)]
/// Settings and functions to calculate induced velocities from vortex lines and vortex panels.
pub struct PotentialTheorySettings {
    /// Distance from a panel, relative to the size of the panel, beyond which a simplified far-field
    /// model is used for the induced velocity
    pub far_field_ratio: Float,
    /// Optional symmetry condition
    pub symmetry_condition: SymmetryCondition,
}

impl Default for PotentialTheorySettings {
    fn default() -> Self {
        Self {
            far_field_ratio: Self::default_far_field_ratio(),
            symmetry_condition: SymmetryCondition::default(),
        }
    }
}

impl PotentialTheorySettings {
    /// Default far-field ratio (5.0)
    pub fn default_far_field_ratio() -> Float { 5.0 }
}

/// Re-exports of commonly used structures, to simplify imports
pub mod prelude {
    pub use super::PotentialTheorySettings;
    pub use super::symmetry_condition::*;
    pub use super::panel::Panel;
}

#[cfg(test)]
mod tests;