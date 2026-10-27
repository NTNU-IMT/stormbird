// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A dynamic stall model for foil section models, based on the Leishman-Beddoes model, but adapted
//! to the stall model in the [foil model](Foil).
//!
//! The foil model computes the lift as a blend between a pre-stall and a post-stall lift curve,
//! weighted by the *amount of stall*, s(α), which varies between 0 (no stall) and 1 (full stall).
//! The dynamic stall model introduces two first order time lags, applied in sequence:
//!
//! 1. A lag in the pressure distribution, applied to the angle of attack, with the time constant
//!    T_p. This gives a lagged angle of attack, α_f.
//! 2. A lag in the boundary layer separation, applied to the amount of stall evaluated at the
//!    lagged angle of attack, s' = s(α_f), with the time constant T_f. This gives the dynamic
//!    amount of stall, s''.
//!
//! The lift is then computed from the same blend as in the foil model, with the curves evaluated at
//! the instantaneous angle of attack, but weighted with s'' rather than s(α).
//!
//! The model is fundamentally a two dimensional model, but it is kept as a separate structure from
//! the foil models, as it has an internal state, i.e., the memory effects for each line segment on
//! a wing. The model is designed to be used in a time stepping simulation, where the solver can
//! evaluate the lift many times per time step. The evaluation is therefore always based on the
//! state from the last completed time step, and does not change the state. The state is updated
//! explicitly with [`DynamicStallModel::commit`] when a time step is completed.

/// Builder for the dynamic stall model
pub mod builder;

#[cfg(test)]
mod tests;

use stormath::{
    type_aliases::Float,
    consts::{PI, TAU},
};

use super::SectionModel;
use super::foil::Foil;

pub use builder::DynamicStallModelBuilder;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
/// The state of the dynamic stall model for a single line segment at a single time step.
pub struct DynamicStallSegmentState {
    /// The instantaneous angle of attack, α
    pub angle_of_attack: Float,
    /// The deficiency function for the lag in the pressure distribution, D_p
    pub pressure_deficiency: Float,
    /// The static amount of stall evaluated at the lagged angle of attack, s'
    pub lagged_angle_amount_of_stall: Float,
    /// The deficiency function for the lag in the boundary layer separation, D_s
    pub separation_deficiency: Float,
    /// The dynamic amount of stall, s'', which is used to compute the lift
    pub amount_of_stall: Float,
}

impl DynamicStallSegmentState {
    /// The state in steady conditions, where both deficiency functions are zero, and the dynamic
    /// amount of stall is equal to the static value.
    pub fn steady(foil: &Foil, angle_of_attack: Float) -> Self {
        let amount_of_stall = foil.amount_of_stall(angle_of_attack);

        Self {
            angle_of_attack,
            pressure_deficiency: 0.0,
            lagged_angle_amount_of_stall: amount_of_stall,
            separation_deficiency: 0.0,
            amount_of_stall,
        }
    }

    /// The lagged angle of attack, α_f = α - D_p, wrapped to the range [-π, π)
    pub fn lagged_angle_of_attack(&self) -> Float {
        wrapped_angle(self.angle_of_attack - self.pressure_deficiency)
    }
}

#[derive(Debug, Clone)]
/// Dynamic stall model for a single wing. Contains both the parameters of the model, and the state
/// from the last completed time step for each line segment on the wing.
pub struct DynamicStallModel {
    /// Non-dimensional time constant for the lag in the pressure distribution, T_p
    pub pressure_time_constant: Float,
    /// Non-dimensional time constant for the lag in the boundary layer separation, T_f
    pub separation_time_constant: Float,
    /// The dimensional time step of the current time step. Must be set before the model is
    /// evaluated, with [`DynamicStallModel::set_time_step`].
    pub time_step: Float,
    /// The state from the last completed time step, for each line segment on the wing
    pub segment_states: Vec<DynamicStallSegmentState>,
    /// Whether the state has been set from a completed time step. If not, the model gives the
    /// static values, and the first completed time step is used to initialize the state.
    pub is_initialized: bool,
}

impl DynamicStallModel {
    /// Returns true if the dynamic stall model can be used together with the section model. This
    /// is the case for all section models that are built on the foil model.
    pub fn is_supported(section_model: &SectionModel) -> bool {
        matches!(
            section_model,
            SectionModel::SingleElementFoil(_) |
            SectionModel::MultiElementFoil(_) |
            SectionModel::ActiveAirFlowFoil(_)
        )
    }

    /// Returns the foil model that the section model uses for the given internal state, or `None`
    /// if the section model is not built on the foil model.
    pub fn foil(section_model: &SectionModel, internal_state: Float) -> Option<Foil> {
        match section_model {
            SectionModel::SingleElementFoil(foil) => Some(foil.clone()),
            SectionModel::MultiElementFoil(varying_foil) => Some(
                varying_foil.get_foil(internal_state)
            ),
            SectionModel::ActiveAirFlowFoil(active_air_flow_foil) => Some(
                active_air_flow_foil.varying_foil.get_foil(internal_state)
            ),
            SectionModel::RotatingCylinder(_) | SectionModel::EffectiveWindSensor => None,
        }
    }

    /// Sets the dimensional time step to use for the next evaluations of the model
    pub fn set_time_step(&mut self, time_step: Float) {
        self.time_step = time_step;
    }

    /// Resets the state, so that the model gives static values until the next completed time step
    pub fn reset(&mut self) {
        self.is_initialized = false;

        for state in self.segment_states.iter_mut() {
            *state = DynamicStallSegmentState::default();
        }
    }

    /// Returns the non-dimensional time step, Δs = 2 V Δt / c, i.e., the number of semi-chords
    /// travelled during the time step.
    pub fn non_dimensional_time_step(&self, velocity: Float, chord_length: Float) -> Float {
        if chord_length > 0.0 {
            2.0 * velocity * self.time_step / chord_length
        } else {
            0.0
        }
    }

    /// Evaluates the state of the model for a single line segment at the current time step, based
    /// on the state from the last completed time step. The model itself is not changed.
    ///
    /// If the model is not initialized, the steady state is returned.
    ///
    /// # Arguments
    /// * `local_index` - the index of the line segment on the wing
    /// * `foil` - the foil model of the line segment, at the current internal state
    /// * `angle_of_attack` - the instantaneous angle of attack
    /// * `velocity` - the magnitude of the velocity at the control point
    /// * `chord_length` - the chord length of the line segment
    pub fn evaluate(
        &self,
        local_index: usize,
        foil: &Foil,
        angle_of_attack: Float,
        velocity: Float,
        chord_length: Float,
    ) -> DynamicStallSegmentState {
        if !self.is_initialized {
            return DynamicStallSegmentState::steady(foil, angle_of_attack);
        }

        let previous = &self.segment_states[local_index];

        let delta_s = self.non_dimensional_time_step(velocity, chord_length);

        // Step 1: pressure lag, applied to the angle of attack
        let (decay, increment_weight) = lag_factors(delta_s, self.pressure_time_constant);

        let angle_increment = wrapped_angle(angle_of_attack - previous.angle_of_attack);

        let pressure_deficiency = previous.pressure_deficiency * decay +
            angle_increment * increment_weight;

        let lagged_angle_of_attack = wrapped_angle(angle_of_attack - pressure_deficiency);

        // Step 2: separation lag, applied to the amount of stall at the lagged angle of attack
        let lagged_angle_amount_of_stall = foil.amount_of_stall(lagged_angle_of_attack);

        let (decay, increment_weight) = lag_factors(delta_s, self.separation_time_constant);

        let separation_deficiency = previous.separation_deficiency * decay +
            (lagged_angle_amount_of_stall - previous.lagged_angle_amount_of_stall) * increment_weight;

        // The discrete lag is not strictly guaranteed to stay within the range of the input
        let amount_of_stall = (lagged_angle_amount_of_stall - separation_deficiency).clamp(0.0, 1.0);

        DynamicStallSegmentState {
            angle_of_attack,
            pressure_deficiency,
            lagged_angle_amount_of_stall,
            separation_deficiency,
            amount_of_stall,
        }
    }

    /// Stores the state for all line segments on the wing from a completed time step. The states
    /// should come from [`DynamicStallModel::evaluate`], using the final solution of the time step.
    ///
    /// # Panics
    /// If the number of states is different from the number of line segments on the wing.
    pub fn commit(&mut self, segment_states: Vec<DynamicStallSegmentState>) {
        if segment_states.len() != self.segment_states.len() {
            panic!(
                "The number of states given to the dynamic stall model ({}) is different from the \
                number of line segments on the wing ({})",
                segment_states.len(),
                self.segment_states.len()
            );
        }

        self.segment_states = segment_states;
        self.is_initialized = true;
    }

    /// The pre-stall lift coefficient, reduced by the dynamic amount of stall
    pub fn lift_coefficient_pre_stall_with_stall_drop_off(
        foil: &Foil,
        angle_of_attack: Float,
        state: &DynamicStallSegmentState
    ) -> Float {
        foil.lift_coefficient_pre_stall_raw(angle_of_attack) * (1.0 - state.amount_of_stall)
    }

    /// The post-stall lift coefficient, weighted by the dynamic amount of stall
    pub fn lift_coefficient_post_stall_with_stall_weight(
        foil: &Foil,
        angle_of_attack: Float,
        state: &DynamicStallSegmentState
    ) -> Float {
        foil.lift_coefficient_post_stall_raw(angle_of_attack) * state.amount_of_stall
    }
}

/// Returns the factors used in the discrete deficiency function for a first order lag:
///
/// D_i = D_{i-1} * exp(-Δs / T) + (x_i - x_{i-1}) * exp(-Δs / (2 T))
///
/// The first value is the decay of the previous deficiency, and the second value is the weight of
/// the increment in the input. A time constant of zero, or less, turns the lag off.
fn lag_factors(delta_s: Float, time_constant: Float) -> (Float, Float) {
    if time_constant <= 0.0 {
        return (0.0, 0.0);
    }

    (
        (-delta_s / time_constant).exp(),
        (-delta_s / (2.0 * time_constant)).exp()
    )
}

/// Wraps an angle to the range [-π, π), so that differences between angles close to ±π are
/// computed as the smallest difference.
fn wrapped_angle(angle: Float) -> Float {
    (angle + PI).rem_euclid(TAU) - PI
}
