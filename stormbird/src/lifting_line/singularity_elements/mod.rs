// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of the necessary singularuty elements for the lifting line method. 

use symmetry_condition::SymmetryCondition;

pub mod vortex_line;
pub mod symmetry_condition;
pub mod panel;

use stormath::type_aliases::Float;

#[derive(Clone, Debug)]
/// Settings and functions to calculate induced velocities from vortex lines and vortex panels.
pub struct PotentialTheorySettings {
    pub far_field_ratio: Float,
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
    pub fn default_far_field_ratio() -> Float { 5.0 }
}

pub mod prelude {
    pub use super::PotentialTheorySettings;
    pub use super::symmetry_condition::*;
    pub use super::panel::Panel;
}

#[cfg(test)]
mod tests;