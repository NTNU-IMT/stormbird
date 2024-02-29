// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Models of two dimensional lift and drag on wing sections, such as foil profiles and rotating 
//! cylinders. 


/// Collection of functions that are useful for multiple section models.
pub mod common_functions;
/// Section model of a foil profile
pub mod foil;
/// Section model of a rotating cylinder, for instance to be used when modelling rotor sails.
pub mod rotating_cylinder;

use serde::{Serialize, Deserialize};

use foil::Foil;
use rotating_cylinder::RotatingCylinder;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Sectional model for a wing, that can be of multple variants
pub enum SectionModel {
    Foil(Foil),
    VaryingFoil(Vec<Foil>),
    RotatingCylinder(RotatingCylinder),
}

#[cfg(test)]
mod tests;