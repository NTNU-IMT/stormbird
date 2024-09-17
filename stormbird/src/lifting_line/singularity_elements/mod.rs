// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of the necessary singularuty elements for the lifting line method. 

use symmetry_condition::SymmetryCondition;

pub mod vortex_line;
pub mod symmetry_condition;
pub mod panel;

#[derive(Clone, Debug)]
/// Settings and functions to calculate induced velocities from vortex lines and vortex panels.
pub struct PotentialTheoryModel {
    pub closeness_error: f64,
    pub far_field_ratio: f64,
    pub symmetry_condition: SymmetryCondition,
}

impl Default for PotentialTheoryModel {
    fn default() -> Self {
        Self {
            closeness_error: f64::MIN_POSITIVE,
            far_field_ratio: Self::default_far_field_ratio(),
            symmetry_condition: SymmetryCondition::default(),
        }
    }
}

impl PotentialTheoryModel {
    pub fn default_far_field_ratio() -> f64 { 5.0 }
}

pub mod prelude {
    pub use super::PotentialTheoryModel;
    pub use super::symmetry_condition::*;
}

#[cfg(test)]
mod tests;