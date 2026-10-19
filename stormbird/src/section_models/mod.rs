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
use stormath::type_aliases::Float;

use foil::Foil;
use varying_foil::VaryingFoil;
use rotating_cylinder::RotatingCylinder;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Sectional model for a wing, that can be of multiple variants
pub enum SectionModel {
    SingleElementFoil(Foil),
    MultiElementFoil(VaryingFoil),
    ActiveAirFlowFoil(VaryingFoil),
    RotatingCylinder(RotatingCylinder),
    EffectiveWindSensor,
}

impl SectionModel {
    pub fn amount_of_flow_separation(&self, angle_of_attack: Float) -> Float {
        match self {
            SectionModel::SingleElementFoil(foil) => foil.amount_of_stall(angle_of_attack),
            SectionModel::MultiElementFoil(varying_foil) => varying_foil.amount_of_stall(angle_of_attack),
            SectionModel::ActiveAirFlowFoil(varying_foil) => varying_foil.amount_of_stall(angle_of_attack),
            SectionModel::RotatingCylinder(_) => 1.0,
            SectionModel::EffectiveWindSensor => 0.0
        }
    }

    pub fn lift_coefficient(&self, angle_of_attack: Float) -> Float {
        todo!();
    }

    pub fn lift_coefficients_linear(&self, angle_of_attack: Float) -> Float {
        todo!()
    }

    pub fn lift_coefficients_pre_stall_with_stall_drop_off(&self) -> Float {
        todo!();
    }

    pub fn lift_coefficients_post_stall_with_stall_weight(&self) -> Float {
        todo!();
    }

    pub fn lift_coefficients_derivatives(&self) -> Float {
        todo!();
    }

    pub fn drag_coefficient(&self) -> Float {
        todo!();
    }

    pub fn from_string(setup_string: &str) -> Result<Self, String> {
        let section_model: SectionModel = serde_json::from_str(setup_string)
            .map_err(|e| format!("Failed to parse section model from string: {}", e))?;
        
        Ok(section_model)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl Default for SectionModel {
    fn default() -> Self {
        SectionModel::SingleElementFoil(Foil::default())
    }
}

#[cfg(test)]
mod tests;