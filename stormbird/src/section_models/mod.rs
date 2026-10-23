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
/// Section model of a foil that uses active air flow, e.g. suction.
pub mod active_air_flow_foil;

use serde::{Serialize, Deserialize};
use stormath::type_aliases::Float;

use foil::Foil;
use varying_foil::VaryingFoil;
use rotating_cylinder::RotatingCylinder;
use active_air_flow_foil::ActiveAirFlowFoil;
use crate::line_force_model::wing_internal_state::{WingInternalState, InternalCompartments};

#[derive(Debug, Clone)]
/// The local flow conditions on a single segment of a wing. Used by section models where the
/// relationship between the wing and the sections depends on the flow conditions along the whole
/// wing, such as when converting the wing internal state or computing the power for a wing.
pub struct SegmentFlowConditions {
    /// The angle of attack at the control point of the segment
    pub angle_of_attack: Float,
    /// The magnitude of the velocity at the control point of the segment
    pub velocity: Float,
    /// The area of the segment, i.e., the chord length times the span length
    pub area: Float,
    /// The length of the segment along the span
    pub span_length: Float,
    /// The non-dimensional location of the control point along the span of the wing, running from
    /// -0.5 to 0.5
    pub non_dim_span_location: Float,
    /// Virtual segments are part of the simulation, but not of the wing itself, for instance when 
    /// modelling end plates. They are therefore not included in the power, or in the flow through
    /// a wing with active air flow.
    pub is_virtual: bool,
    /// The index of the internal compartment the segment belongs to. Only used by section models
    /// with an internal flow model.
    pub compartment_index: usize,
}

impl SegmentFlowConditions {
    /// The chord length of the segment
    pub fn chord_length(&self) -> Float {
        if self.span_length > 0.0 {
            self.area / self.span_length
        } else {
            0.0
        }
    }
}

/// The integral from 0 to `x` of the sigmoid function in
/// [`stormath::transition_functions::sigmoid_zero_to_one`], with the same definition of `x0` and
/// `transition_range`.
///
/// The integral of a logistic sigmoid is the softplus function, which is evaluated in a
/// numerically stable way. If the transition range is zero or negative, the sigmoid is treated as
/// a step function.
pub fn integrated_sigmoid_from_zero(x: Float, x0: Float, transition_range: Float) -> Float {
    if transition_range <= 0.0 {
        return (x - x0).max(0.0) - (-x0).max(0.0);
    }

    // Same slope constant as in `sigmoid_zero_to_one`
    let slope = 4.5951212 / transition_range;

    let softplus = |z: Float| z.max(0.0) + (-z.abs()).exp().ln_1p();

    (softplus(slope * (x - x0)) - softplus(-slope * x0)) / slope
}

/// Integrates a power coefficient over the segments of a wing: P = sum(c_P * 0.5 * density * A * U^3).
/// Virtual segments are not included.
pub fn integrate_power_coefficient(
    segments: &[SegmentFlowConditions],
    section_states: &[Float],
    density: Float,
    power_coefficient: impl Fn(Float) -> Float,
) -> Float {
    segments.iter().zip(section_states)
        .filter(|(segment, _)| !segment.is_virtual)
        .map(|(segment, &section_state)| {
            power_coefficient(section_state) *
                0.5 * density * segment.area * segment.velocity.powi(3)
        }).sum()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Sectional model for a wing, that can be of multiple variants
pub enum SectionModel {
    /// A single-element foil with fixed properties
    SingleElementFoil(Foil),
    /// A foil where the properties vary with the internal state, e.g., a multi-element foil with a
    /// flap angle that can be changed
    MultiElementFoil(VaryingFoil),
    /// A foil with active air flow, e.g., a suction sail
    ActiveAirFlowFoil(ActiveAirFlowFoil),
    /// A rotating cylinder, e.g., a rotor sail
    RotatingCylinder(RotatingCylinder),
    /// A section that gives no forces, used to measure the effective wind at the location of a wing
    EffectiveWindSensor,
}

impl std::fmt::Display for SectionModel {
    /// Formats the model as a JSON string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| std::fmt::Error)?;

        write!(f, "{}", json)
    }
}

impl SectionModel {
    /// Computes the lift coefficient. 
    /// 
    /// # Arguments
    /// - `angle_of_attack`: the angle of attack, relevant if the sectional model is a foil, with or
    ///   without active flow control and flap
    /// - `internal_state`: the internal state to use when calculating the forces. The meaning of 
    ///   the number varies depending on the which section model is used. For a multi-element foil, it
    ///   will typically be the flap angle (or a number implicitly setting the flap and leading edge 
    ///   slot angle). For a foil with active flow control it will typically be either the power 
    ///   coefficient or the non-dimensional suction rate (the flap setting typically implicitly set 
    ///   from the same data). For a rotating cylinder, it will be the spin ratio
    pub fn lift_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.lift_coefficient(angle_of_attack),
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.lift_coefficient(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(rotating_cylinder) => rotating_cylinder.lift_coefficient(
                internal_state
            ),
            Self::EffectiveWindSensor => 0.0
        }
    }

    /// Returns the linear lift coefficient, i.e., the lift without stall effects
    pub fn lift_coefficient_linear(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.lift_coefficient_linear(angle_of_attack),
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient_linear(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.lift_coefficient_linear(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(cylinder) => cylinder.lift_coefficient(internal_state),
            Self::EffectiveWindSensor => 0.0,
        }
    }

    /// Returns the pre-stall lift coefficient, reduced by the amount of stall
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
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.lift_coefficient_pre_stall_with_stall_drop_off(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(cylinder) => cylinder.lift_coefficient(internal_state),
            Self::EffectiveWindSensor => 0.0,
        }
    }

    /// Returns the post-stall lift coefficient, weighted by the amount of stall
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
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.lift_coefficient_post_stall_with_stall_weight(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(_) => 0.0,
            Self::EffectiveWindSensor => 0.0,
        }
    }

    /// Returns the initial slope of the linear lift coefficient
    pub fn lift_coefficient_derivative(&self, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.cl_initial_slope,
            Self::MultiElementFoil(varying_foil) => varying_foil.lift_coefficient_derivative(internal_state),
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.lift_coefficient_derivative(internal_state),
            Self::RotatingCylinder(_) => 0.0,
            Self::EffectiveWindSensor => 0.0,
        }
    }

    /// Returns the drag coefficient
    pub fn drag_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(foil) => foil.drag_coefficient(angle_of_attack),
            Self::MultiElementFoil(varying_foil) => varying_foil.drag_coefficient(
                angle_of_attack, internal_state
            ),
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.drag_coefficient(
                angle_of_attack, internal_state
            ),
            Self::RotatingCylinder(cylinder) => cylinder.drag_coefficient(internal_state),
            Self::EffectiveWindSensor => 0.0,
        }
    }


    /// Computes the power coefficient, c_P = P / (0.5 * density * A * U^3), where A is the area of
    /// the section and U is the local velocity. Section models that do not need power return zero.
    pub fn power_coefficient(&self, internal_state: Float) -> Float {
        match self {
            Self::SingleElementFoil(_) => 0.0,
            Self::MultiElementFoil(_) => 0.0,
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.power_coefficient(
                internal_state
            ),
            Self::RotatingCylinder(cylinder) => cylinder.power_coefficient(internal_state),
            Self::EffectiveWindSensor => 0.0,
        }
    }

    /// Converts the internal state of a wing to the internal state of the section model on each 
    /// segment of the wing.
    ///
    /// For most section models, the value of the wing state at the location of each segment is 
    /// used directly, or with a local conversion, such as from revolutions per second to spin ratio
    /// for a rotating cylinder. Active air flow foils can need information about the whole wing.
    pub fn section_internal_states(
        &self,
        wing_state: &WingInternalState,
        segments: &[SegmentFlowConditions],
        compartments: &InternalCompartments,
        density: Float
    ) -> Vec<Float> {
        match self {
            Self::SingleElementFoil(_) | Self::MultiElementFoil(_) | Self::EffectiveWindSensor => {
                segments.iter().map(|segment| {
                    wing_state.value_at(segment.non_dim_span_location)
                }).collect()
            },
            Self::RotatingCylinder(_) => {
                segments.iter().map(|segment| {
                    RotatingCylinder::spin_ratio_from_revolutions_per_second(
                        wing_state.value_at(segment.non_dim_span_location),
                        segment.chord_length(),
                        segment.velocity
                    )
                }).collect()
            },
            Self::ActiveAirFlowFoil(active_air_flow_foil) => {
                active_air_flow_foil.section_internal_states(wing_state, segments, compartments, density)
            },
        }
    }

    /// Computes the power needed to operate a wing using this section model.
    ///
    /// For most section models, this is the power coefficient integrated over the wing. However,
    /// some models need information about the whole wing to compute the power, such as an active
    /// air flow foil where the power is given by a fan model that depends on the total flow.
    ///
    /// # Arguments
    /// * `wing_state` - the internal state of the wing
    /// * `segments` - the flow conditions on each segment of the wing
    /// * `compartments` - the internal compartments of the wing
    /// * `section_states` - the internal state of the section model on each segment
    /// * `density` - the density of the fluid
    pub fn wing_power(
        &self,
        wing_state: &WingInternalState,
        segments: &[SegmentFlowConditions],
        compartments: &InternalCompartments,
        section_states: &[Float],
        density: Float
    ) -> Float {
        match self {
            Self::ActiveAirFlowFoil(active_air_flow_foil) => active_air_flow_foil.wing_power(
                wing_state, segments, compartments, section_states, density
            ),
            _ => integrate_power_coefficient(
                segments, section_states, density, |section_state| {
                    self.power_coefficient(section_state)
                }
            ),
        }
    }

    /// Returns true if the section model uses internal compartments, i.e., if it has an internal
    /// flow model.
    pub fn uses_internal_compartments(&self) -> bool {
        match self {
            Self::ActiveAirFlowFoil(active_air_flow_foil) => {
                active_air_flow_foil.uses_internal_compartments()
            },
            _ => false,
        }
    }

    /// Creates a section model from a JSON string
    pub fn from_string(setup_string: &str) -> Result<Self, String> {
        let section_model: SectionModel = serde_json::from_str(setup_string)
            .map_err(|e| format!("Failed to parse section model from string: {}", e))?;
        
        Ok(section_model)
    }
}

impl Default for SectionModel {
    fn default() -> Self {
        SectionModel::SingleElementFoil(Foil::default())
    }
}

#[cfg(test)]
mod tests;
