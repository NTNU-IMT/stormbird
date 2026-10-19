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
    /// Computes the lift coefficient. 
    /// 
    /// # Arguments
    /// - `angle_of_attack`: the angle of attack, relevant if the sectional model is a foil, with or
    /// without active flow control and flap
    /// - `internal_state`: the internal state to use when calculating the forces. The meaning of 
    /// the number varies depending on the which section model is used. For a multi-element foil, it
    /// will typically be the flap angle (or a number implicitly setting the flap and leading edge 
    /// slot angle). For a foil with active flow control it will typically be either the power 
    /// coefficient or the non-dimensional suction rate (the flap setting typically implicitly set 
    /// from the same data). For a rotating cylinder, it will be the spin ratio
    pub fn lift_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.lift_coefficient(angle_of_attack),
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(varying_foil) => varying_foil.lift_coefficient(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(rotating_cylinder) => rotating_cylinder.lift_coefficient(
                internal_state
            ),
            Self::EffectiveWindSensor => 0.0
        }
    }

    pub fn lift_coefficient_linear(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.lift_coefficient_linear(angle_of_attack),
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient_linear(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(varying_foil) => varying_foil.lift_coefficient_linear(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(cylinder) => cylinder.lift_coefficient(internal_state),
            Self::EffectiveWindSensor => 0.0,
        }
    }

    pub fn lift_coefficient_pre_stall_with_stall_drop_off(
        &self, 
        angle_of_attack: Float, 
        internal_state: Float
    ) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.lift_coefficient_pre_stall_with_stall_drop_off(
                angle_of_attack
            ),
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient_pre_stall_with_stall_drop_off(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(varying_foil) => varying_foil.lift_coefficient_pre_stall_with_stall_drop_off(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(cylinder) => cylinder.lift_coefficient(internal_state),
            Self::EffectiveWindSensor => 0.0,
        }
    }

    pub fn lift_coefficient_post_stall_with_stall_weight(
        &self, 
        angle_of_attack: Float, 
        internal_state: Float
    ) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.lift_coefficient_post_stall_with_stall_weight(
                angle_of_attack
            ),
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient_post_stall_with_stall_weight(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(varying_foil) => varying_foil.lift_coefficient_post_stall_with_stall_weight(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(_) => 0.0,
            Self::EffectiveWindSensor => 0.0,
        }
    }

    pub fn lift_coefficient_derivative(&self, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.cl_initial_slope,
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient_derivative(internal_state),
            Self::ActiveAirFlowFoil(varying_foil) => varying_foil.lift_coefficient_derivative(internal_state),
            Self::RotatingCylinder(_) => 0.0,
            Self::EffectiveWindSensor => 0.0,
        }
    }

    pub fn drag_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.drag_coefficient(angle_of_attack),
            Self::MultiElementFoil(varying_foil) => varying_foil.drag_coefficient(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(varying_foil) => varying_foil.drag_coefficient(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(cylinder) => cylinder.drag_coefficient(internal_state),
            Self::EffectiveWindSensor => 0.0,
        }
    }

    pub fn added_mass_factor(&self, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.added_mass_factor,
            Self::MultiElementFoil(varying_foil) => varying_foil.get_foil(internal_state).added_mass_factor,
            Self::ActiveAirFlowFoil(varying_foil) => varying_foil.get_foil(internal_state).added_mass_factor,
            Self::RotatingCylinder(cylinder) => cylinder.added_mass_factor,
            Self::EffectiveWindSensor => 0.0
        }
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
