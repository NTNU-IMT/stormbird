// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Models of two dimensional lift and drag on wing sections, such as foil profiles and rotating 
//! cylinders. 

/// Section model of a foil profile
pub mod foil;
/// Section model of a foil profile where the parameters can vary depending on an internal state
pub mod varying_foil;
/// Section model of a rotating cylinder, for instance to be used when modelling rotor sails.
pub mod rotating_cylinder;

use serde::{Serialize, Deserialize};

use foil::Foil;
use varying_foil::VaryingFoil;
use rotating_cylinder::RotatingCylinder;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Sectional model for a wing, that can be of multiple variants
pub enum SectionModel {
    Foil(Foil),
    VaryingFoil(VaryingFoil),
    RotatingCylinder(RotatingCylinder),
}

impl SectionModel {
    pub fn amount_of_flow_separation(&self, angle_of_attack: f64) -> f64 {
        match self {
            SectionModel::Foil(foil) => foil.amount_of_stall(angle_of_attack),
            SectionModel::VaryingFoil(varying_foil) => varying_foil.amount_of_stall(angle_of_attack),
            SectionModel::RotatingCylinder(_) => 1.0,
        }
    }
}

impl Default for SectionModel {
    fn default() -> Self {
        SectionModel::Foil(Foil::default())
    }
}

#[cfg(test)]
mod tests;