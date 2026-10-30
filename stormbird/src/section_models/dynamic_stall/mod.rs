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
//! The lift and the drag are then computed from the same blends as in the foil model, with the
//! curves evaluated at the instantaneous angle of attack, but weighted with s'' rather than s(α).
//! The model itself does not compute the forces. Instead, it gives the effects that enter the force
//! coefficients, as a [`DynamicStallInput`](crate::section_models::dynamic_stall::DynamicStallInput),
//! which is combined with the section model by the user of the model.
//!
//! In addition, the model includes the extra lift from a leading edge vortex. The vortex is fed by
//! the lift that is lost due to the dynamic stall, i.e., s'' times the difference between the
//! pre-stall and post-stall lift. The feed is accumulated with a deficiency function with the time
//! constant T_v, but only while the vortex is above the foil. The vortex is created when the lagged
//! angle of attack exceeds the mean stall angle, i.e., when s' > 0.5, and passes the trailing edge
//! after the non-dimensional time T_vl. After that, the vortex lift only decays. The vortex lift is
//! treated as part of the stall related lift, i.e., added to the post-stall lift.
//!
//! **Experimental**: the model can also apply a first order lag to the internal state of the
//! section model, with the time constant T_i. This is applied before all the other steps, so that
//! the section model is evaluated with the lagged internal state. It is mostly intended for
//! rotating cylinders, where the internal state is the spin ratio, and is the only part of the
//! model that is used for section models that are not built on the foil model. The lag is off by
//! default.
//!
//! The model is fundamentally a two dimensional model, but it is kept as a separate structure from
//! the section models, as it has an internal state, i.e., the memory effects for each line segment
//! on a wing. The model is designed to be used in a time stepping simulation, where the solver can
//! evaluate the forces many times per time step. The evaluation is therefore always based on the
//! state from the last completed time step, and does not change the state. The state is updated
//! explicitly with
//! [`DynamicStallModel::commit`](crate::section_models::dynamic_stall::DynamicStallModel::commit)
//! when a time step is completed.

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
///
/// For section models that are not built on the foil model, only the internal state, the internal
/// state deficiency and the angle of attack are used. The other values are then zero.
pub struct DynamicStallSegmentState {
    /// The instantaneous internal state of the section model
    pub internal_state: Float,
    /// The deficiency function for the lag in the internal state, D_i
    pub internal_state_deficiency: Float,
    /// The instantaneous angle of attack, α
    pub angle_of_attack: Float,
    /// The deficiency function for the lag in the pressure distribution, D_p
    pub pressure_deficiency: Float,
    /// The static amount of stall evaluated at the lagged angle of attack, s'
    pub lagged_angle_amount_of_stall: Float,
    /// The deficiency function for the lag in the boundary layer separation, D_s
    pub separation_deficiency: Float,
    /// The dynamic amount of stall, s'', which is used to compute the lift and the drag
    pub amount_of_stall: Float,
    /// The lift coefficient that feeds the leading edge vortex, i.e., the lift lost due to the
    /// dynamic stall: s'' (C_L,pre-stall - C_L,post-stall)
    pub vortex_feed: Float,
    /// The non-dimensional time since the leading edge vortex was created, τ_v. Is zero when the
    /// lagged angle of attack is below the stall angle.
    pub vortex_time: Float,
    /// The lift coefficient from the leading edge vortex, C_L,v
    pub vortex_lift: Float,
}

impl DynamicStallSegmentState {
    /// The state in steady conditions, where all deficiency functions are zero, and the dynamic
    /// amount of stall is equal to the static value. There is no vortex lift, and if the foil is
    /// stalled, the leading edge vortex is assumed to have passed the trailing edge a long time ago.
    pub fn steady(section_model: &SectionModel, angle_of_attack: Float, internal_state: Float) -> Self {
        let Some(foil) = section_model.foil(internal_state) else {
            return Self {
                internal_state,
                angle_of_attack,
                ..Default::default()
            };
        };

        let amount_of_stall = foil.amount_of_stall(angle_of_attack);

        let vortex_time = if vortex_onset(amount_of_stall) {
            Float::INFINITY
        } else {
            0.0
        };

        Self {
            internal_state,
            internal_state_deficiency: 0.0,
            angle_of_attack,
            pressure_deficiency: 0.0,
            lagged_angle_amount_of_stall: amount_of_stall,
            separation_deficiency: 0.0,
            amount_of_stall,
            vortex_feed: vortex_feed(&foil, angle_of_attack, amount_of_stall),
            vortex_time,
            vortex_lift: 0.0,
        }
    }

    /// The lagged internal state, x - D_i
    pub fn lagged_internal_state(&self) -> Float {
        self.internal_state - self.internal_state_deficiency
    }

    /// The lagged angle of attack, α_f = α - D_p, wrapped to the range [-π, π)
    pub fn lagged_angle_of_attack(&self) -> Float {
        wrapped_angle(self.angle_of_attack - self.pressure_deficiency)
    }

    /// The parts of the state that enter the force coefficients, i.e., the lagged internal state,
    /// the dynamic amount of stall and the vortex lift
    pub fn input(&self) -> DynamicStallInput {
        DynamicStallInput {
            internal_state: Some(self.lagged_internal_state()),
            amount_of_stall: Some(self.amount_of_stall),
            vortex_lift: self.vortex_lift,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
/// The effects of the dynamic stall model that enter the force coefficients on a single line
/// segment.
///
/// The lift and the drag are computed from the section model, with the internal state and the
/// amount of stall given here, and the vortex lift is added to the lift. Note that the lift and
/// drag curves in the section model are always evaluated at the instantaneous angle of attack. The
/// lagged angle of attack is only used internally in the dynamic stall model, to compute the amount
/// of stall.
pub struct DynamicStallInput {
    /// The internal state to use in the section model for the force coefficients. `None` means
    /// that the instantaneous internal state is used. Not used for the power.
    pub internal_state: Option<Float>,
    /// The amount of stall to use in the section model. `None` means that the static amount of
    /// stall from the section model itself is used. Ignored for section models that are not built
    /// on the foil model.
    pub amount_of_stall: Option<Float>,
    /// The lift coefficient from the leading edge vortex, which is treated as part of the stall
    /// related lift, i.e., added to the post-stall lift.
    pub vortex_lift: Float,
}

impl DynamicStallInput {
    /// The input in steady conditions, or for wings without a dynamic stall model: the
    /// instantaneous internal state, the static amount of stall from the section model, and no
    /// vortex lift
    pub fn steady() -> Self {
        Self::default()
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
    /// Non-dimensional time constant for the decay of the leading edge vortex lift, T_v
    pub vortex_time_constant: Float,
    /// Non-dimensional time it takes for the leading edge vortex to pass the trailing edge, T_vl
    pub vortex_passage_time: Float,
    /// **Experimental**. Non-dimensional time constant for the lag in the internal state of the
    /// section model, T_i
    pub internal_state_time_constant: Float,
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
    /// Returns true if the stall part of the dynamic stall model, i.e., everything except the lag
    /// in the internal state, is used together with the section model. This is the case for all
    /// section models that are built on the foil model.
    pub fn uses_foil_stall_model(section_model: &SectionModel) -> bool {
        matches!(
            section_model,
            SectionModel::SingleElementFoil(_) |
            SectionModel::MultiElementFoil(_) |
            SectionModel::ActiveAirFlowFoil(_)
        )
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

    /// The discrete deficiency function, which is used for all the first order lags in the model:
    ///
    /// D_i = D_{i-1} * exp(-Δs / T) + Δx_i * exp(-Δs / (2 T))
    ///
    /// For a lagged signal, the increment is the change in the input since the last time step,
    /// Δx_i = x_i - x_{i-1}, and the lagged signal is x_i - D_i. A time constant of zero, or less,
    /// turns the lag off, i.e., the deficiency function is always zero.
    ///
    /// # Arguments
    /// * `previous_deficiency` - the value of the deficiency function at the last time step, D_{i-1}
    /// * `increment` - the increment in the input, Δx_i
    /// * `delta_s` - the non-dimensional time step, Δs
    /// * `time_constant` - the non-dimensional time constant of the lag, T
    pub fn deficiency_function(
        previous_deficiency: Float,
        increment: Float,
        delta_s: Float,
        time_constant: Float
    ) -> Float {
        if time_constant <= 0.0 {
            return 0.0;
        }

        previous_deficiency * (-delta_s / time_constant).exp() +
            increment * (-delta_s / (2.0 * time_constant)).exp()
    }

    /// Evaluates the state of the model for a single line segment at the current time step, based
    /// on the state from the last completed time step. The model itself is not changed.
    ///
    /// If the model is not initialized, the steady state is returned.
    ///
    /// # Arguments
    /// * `local_index` - the index of the line segment on the wing
    /// * `section_model` - the section model of the wing
    /// * `angle_of_attack` - the instantaneous angle of attack
    /// * `internal_state` - the instantaneous internal state of the section model
    /// * `velocity` - the magnitude of the velocity at the control point
    /// * `chord_length` - the chord length of the line segment
    pub fn evaluate(
        &self,
        local_index: usize,
        section_model: &SectionModel,
        angle_of_attack: Float,
        internal_state: Float,
        velocity: Float,
        chord_length: Float,
    ) -> DynamicStallSegmentState {
        if !self.is_initialized {
            return DynamicStallSegmentState::steady(section_model, angle_of_attack, internal_state);
        }

        let previous = &self.segment_states[local_index];

        let delta_s = self.non_dimensional_time_step(velocity, chord_length);

        // Step 0: internal state lag. The rest of the model uses the lagged internal state.
        let internal_state_deficiency = Self::deficiency_function(
            previous.internal_state_deficiency,
            internal_state - previous.internal_state,
            delta_s,
            self.internal_state_time_constant
        );

        let lagged_internal_state = internal_state - internal_state_deficiency;

        let Some(foil) = section_model.foil(lagged_internal_state) else {
            return DynamicStallSegmentState {
                internal_state,
                internal_state_deficiency,
                angle_of_attack,
                ..Default::default()
            };
        };

        // Step 1: pressure lag, applied to the angle of attack
        let pressure_deficiency = Self::deficiency_function(
            previous.pressure_deficiency,
            wrapped_angle(angle_of_attack - previous.angle_of_attack),
            delta_s,
            self.pressure_time_constant
        );

        let lagged_angle_of_attack = wrapped_angle(angle_of_attack - pressure_deficiency);

        // Step 2: separation lag, applied to the amount of stall at the lagged angle of attack
        let lagged_angle_amount_of_stall = foil.amount_of_stall(lagged_angle_of_attack);

        let separation_deficiency = Self::deficiency_function(
            previous.separation_deficiency,
            lagged_angle_amount_of_stall - previous.lagged_angle_amount_of_stall,
            delta_s,
            self.separation_time_constant
        );

        // The discrete lag is not strictly guaranteed to stay within the range of the input
        let amount_of_stall = (lagged_angle_amount_of_stall - separation_deficiency).clamp(0.0, 1.0);

        // Step 4: vortex feed, i.e., the lift lost due to the dynamic stall
        let vortex_feed = vortex_feed(&foil, angle_of_attack, amount_of_stall);

        // Step 5: vortex onset and vortex time
        let vortex_time = if vortex_onset(lagged_angle_amount_of_stall) {
            previous.vortex_time + delta_s
        } else {
            0.0
        };

        // Step 6: vortex lift, which is only fed while the vortex is above the foil. Otherwise, the
        // increment is zero, so that the vortex lift only decays.
        let vortex_is_above_foil = vortex_time > 0.0 && vortex_time <= self.vortex_passage_time;

        let vortex_feed_increment = if vortex_is_above_foil {
            vortex_feed - previous.vortex_feed
        } else {
            0.0
        };

        let vortex_lift = Self::deficiency_function(
            previous.vortex_lift,
            vortex_feed_increment,
            delta_s,
            self.vortex_time_constant
        );

        DynamicStallSegmentState {
            internal_state,
            internal_state_deficiency,
            angle_of_attack,
            pressure_deficiency,
            lagged_angle_amount_of_stall,
            separation_deficiency,
            amount_of_stall,
            vortex_feed,
            vortex_time,
            vortex_lift,
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
}

/// The lift coefficient that feeds the leading edge vortex, i.e., the difference between the
/// pre-stall lift and the dynamic lift, which simplifies to s'' (C_L,pre-stall - C_L,post-stall)
fn vortex_feed(foil: &Foil, angle_of_attack: Float, amount_of_stall: Float) -> Float {
    amount_of_stall * (
        foil.lift_coefficient_pre_stall_raw(angle_of_attack) -
        foil.lift_coefficient_post_stall_raw(angle_of_attack)
    )
}

/// The onset criterion for the leading edge vortex: the lagged angle of attack is above the mean
/// stall angle, which is the same as the static amount of stall at the lagged angle being above 0.5
fn vortex_onset(lagged_angle_amount_of_stall: Float) -> bool {
    lagged_angle_amount_of_stall > 0.5
}

/// Wraps an angle to the range [-π, π), so that differences between angles close to ±π are
/// computed as the smallest difference.
fn wrapped_angle(angle: Float) -> Float {
    (angle + PI).rem_euclid(TAU) - PI
}
