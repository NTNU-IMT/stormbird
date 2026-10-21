
use serde::{Serialize, Deserialize};
use stormath::type_aliases::Float;
use stormath::consts::MIN_POSITIVE;
use stormath::solvers::secant_solver;

use super::varying_foil::VaryingFoil;

pub mod internal_flow;

use internal_flow::{InternalFlow, InternalFlowInput};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines how the internal state set on the wing is converted to the internal state used by the
/// foil model on each segment.
///
/// **Note**: the meaning of the `internal_state_data` in the varying foil model depends on the
/// variant, as described below.
///
/// For all variants, the sign of the wing state sets the direction of the suction/flap, and is
/// preserved in the section internal states. The magnitude of the power or suction coefficient is
/// therefore given by the absolute value of the wing state.
pub enum InternalStateConversion {
    /// The wing state is the suction coefficient, C_Q = Q / (U * A), which is passed directly to
    /// the foil model. The foil data must be given as a function of C_Q.
    SuctionCoefficientDirect,
    /// The wing state is the power, which is converted to a power coefficient on each segment as
    /// C_P = P / (0.5 * density * A_wing * U^3), where U is the local velocity. The foil data must
    /// be given as a function of C_P.
    PowerToPowerCoefficient,
    /// The wing state is the fan input power, which is converted to a suction coefficient on each
    /// segment by solving for the internal suction that gives the specified power, using the
    /// internal flow model, including the fan efficiency. The foil data must be given as a
    /// function of C_Q.
    PowerToInternalFlow(InternalFlow),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
/// A foil section with active air flow, e.g., suction.
///
/// The internal state of the foil model is either the suction coefficient or the power
/// coefficient, depending on the internal state conversion.
pub struct ActiveAirFlowFoil {
    pub varying_foil: VaryingFoil,
    pub internal_state_conversion: InternalStateConversion,
}

#[derive(Debug, Clone)]
/// The local flow conditions on a single segment of a wing, as needed when converting the wing
/// state to the internal state of the foil model
pub struct SegmentFlowConditions {
    pub angle_of_attack: Float,
    pub velocity: Float,
    pub area: Float,
}

impl ActiveAirFlowFoil {
    pub fn lift_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.varying_foil.lift_coefficient(angle_of_attack, internal_state)
    }

    pub fn lift_coefficient_linear(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.varying_foil.lift_coefficient_linear(angle_of_attack, internal_state)
    }

    pub fn lift_coefficient_pre_stall_with_stall_drop_off(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.varying_foil.lift_coefficient_pre_stall_with_stall_drop_off(angle_of_attack, internal_state)
    }

    pub fn lift_coefficient_post_stall_with_stall_weight(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.varying_foil.lift_coefficient_post_stall_with_stall_weight(angle_of_attack, internal_state)
    }

    pub fn lift_coefficient_derivative(&self, internal_state: Float) -> Float {
        self.varying_foil.lift_coefficient_derivative(internal_state)
    }

    pub fn drag_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.varying_foil.drag_coefficient(angle_of_attack, internal_state)
    }

    pub fn added_mass_factor(&self, internal_state: Float) -> Float {
        self.varying_foil.get_foil(internal_state).added_mass_factor
    }

    /// Computes the power coefficient, c_P = P / (0.5 * density * A * U^3), from the internal
    /// state of the foil model. The power is always positive, as the sign of the internal state
    /// only determines the direction of the suction/flap.
    pub fn power_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        match &self.internal_state_conversion {
            InternalStateConversion::SuctionCoefficientDirect => 0.0,
            InternalStateConversion::PowerToPowerCoefficient => internal_state.abs(),
            InternalStateConversion::PowerToInternalFlow(internal_flow) => {
                let suction_coefficient = internal_state.abs();

                if suction_coefficient < MIN_POSITIVE {
                    return 0.0;
                }

                let direction = internal_state.signum();

                let lift_coefficient = direction * self.lift_coefficient(
                    angle_of_attack, internal_state
                );

                // P_flow = s * Q = (s / q) * C_Q * 0.5 * density * A * U^3
                let flow_power_coefficient = suction_coefficient * internal_flow.internal_suction_coefficient(
                    suction_coefficient, lift_coefficient
                );

                internal_flow.fan_power_from_flow_power(flow_power_coefficient)
            },
        }
    }

    /// Converts the internal state set on the wing to the internal state of the foil model on each
    /// segment, according to the internal state conversion.
    ///
    /// The sign of the wing state is preserved in the section internal states, so that it can be
    /// used to set the direction of the suction/flap.
    pub fn section_internal_states(
        &self,
        wing_state: Float,
        segments: &[SegmentFlowConditions],
        density: Float
    ) -> Vec<Float> {
        match &self.internal_state_conversion {
            InternalStateConversion::SuctionCoefficientDirect => {
                vec![wing_state; segments.len()]
            },
            InternalStateConversion::PowerToPowerCoefficient => {
                // The power is distributed to each segment proportional to the segment area:
                // P_segment = P * A_segment / A_wing. The segment power coefficient is then
                // P_segment / (0.5 * A_segment * density * U^3), where the segment area cancels,
                // giving P / (0.5 * A_wing * density * U^3).
                let wing_area: Float = segments.iter().map(|segment| segment.area).sum();

                segments.iter().map(|segment| {
                    if wing_area > MIN_POSITIVE && segment.velocity > MIN_POSITIVE {
                        wing_state / (0.5 * wing_area * density * segment.velocity.powi(3))
                    } else {
                        0.0
                    }
                }).collect()
            },
            InternalStateConversion::PowerToInternalFlow(internal_flow) => {
                self.suction_coefficients_from_power(internal_flow, wing_state, segments, density)
            },
        }
    }

    /// Estimates the wing state that gives a specified section internal state. This is the inverse
    /// of [`section_internal_states`](ActiveAirFlowFoil::section_internal_states), assuming the
    /// section internal state is uniform over the wing, and that the flow conditions can be
    /// represented by a single angle of attack and velocity.
    ///
    /// For the power based conversions, the power is calculated from the power coefficient of the
    /// section model: P = sign(state) * c_P(alpha, state) * 0.5 * density * A_wing * U^3.
    pub fn wing_state_from_section_state(
        &self,
        section_state: Float,
        angle_of_attack: Float,
        velocity: Float,
        wing_area: Float,
        density: Float
    ) -> Float {
        match &self.internal_state_conversion {
            InternalStateConversion::SuctionCoefficientDirect => section_state,
            InternalStateConversion::PowerToPowerCoefficient |
            InternalStateConversion::PowerToInternalFlow(_) => {
                let power_coefficient = self.power_coefficient(angle_of_attack, section_state);

                section_state.signum() * power_coefficient *
                    0.5 * density * wing_area * velocity.abs().powi(3)
            },
        }
    }

    /// Solves for the magnitude of the suction coefficient on a single segment, for a given
    /// internal suction and suction direction.
    ///
    /// The suction coefficient is both the input to the foil model and the output of the internal
    /// flow model, through the lift coefficient. It is therefore found by solving for the value
    /// where the two are consistent: C_Q = f(C_L(alpha, C_Q), s)
    ///
    /// The foil model is evaluated with the internal state `direction * C_Q`, and the lift
    /// coefficient given to the internal flow model is multiplied with the direction, so that it is
    /// positive towards the suction side.
    ///
    /// As the sign of the internal state is reserved for the direction, segments where the
    /// internal flow model predicts outflow are assumed to have zero flow. The returned value is
    /// therefore never negative.
    pub fn segment_suction_coefficient(
        &self,
        internal_flow: &InternalFlow,
        segment: &SegmentFlowConditions,
        density: Float,
        internal_suction: Float,
        direction: Float,
    ) -> Float {
        if segment.velocity < MIN_POSITIVE || segment.area < MIN_POSITIVE {
            return 0.0;
        }

        let suction_coefficient_from_flow_model = |suction_coefficient: Float| {
            let input = InternalFlowInput {
                lift_coefficient: direction * self.lift_coefficient(
                    segment.angle_of_attack, direction * suction_coefficient
                ),
                segment_area: segment.area,
                density,
                velocity: segment.velocity,
                internal_suction,
            };

            internal_flow.suction_coefficient(&input).max(0.0)
        };

        let settings = internal_flow.solver_settings.segment_solver_settings();

        let first_guess = suction_coefficient_from_flow_model(0.0);

        // A higher suction coefficient increases the lift, and therefore the external suction,
        // which reduces the flow. No inflow at zero suction coefficient therefore means no inflow
        // at all.
        if first_guess < MIN_POSITIVE {
            return 0.0;
        }

        let second_guess = suction_coefficient_from_flow_model(first_guess);

        // The secant solver divides by the difference in the residual between the two guesses,
        // which is zero if the foil model is insensitive to the suction coefficient
        if (second_guess - first_guess).abs() < settings.tolerance {
            return second_guess;
        }

        secant_solver(
            |suction_coefficient| {
                suction_coefficient_from_flow_model(suction_coefficient) - suction_coefficient
            },
            first_guess,
            second_guess,
            &settings
        ).max(0.0)
    }

    /// Returns the magnitude of the suction coefficient on all segments, for a given internal
    /// suction and suction direction
    pub fn suction_coefficients(
        &self,
        internal_flow: &InternalFlow,
        segments: &[SegmentFlowConditions],
        density: Float,
        internal_suction: Float,
        direction: Float,
    ) -> Vec<Float> {
        segments.iter().map(|segment| {
            self.segment_suction_coefficient(
                internal_flow, segment, density, internal_suction, direction
            )
        }).collect()
    }

    /// Returns the total volume flow into the wing, summed over all segments, for a given internal
    /// suction and suction direction.
    pub fn total_volume_flow(
        &self,
        internal_flow: &InternalFlow,
        segments: &[SegmentFlowConditions],
        density: Float,
        internal_suction: Float,
        direction: Float,
    ) -> Float {
        segments.iter().map(|segment| {
            let suction_coefficient = self.segment_suction_coefficient(
                internal_flow, segment, density, internal_suction, direction
            );

            suction_coefficient * segment.velocity * segment.area
        }).sum()
    }

    /// Solves for the internal suction that gives the specified fan input power, for a given
    /// suction direction.
    ///
    /// All segments are assumed to share the same internal volume, so that the power delivered to
    /// the flow is computed from the total volume flow: P_flow = s * sum(Q) = fan_efficiency * P.
    /// Returns zero if the power is zero or negative.
    pub fn internal_suction_from_power(
        &self,
        internal_flow: &InternalFlow,
        fan_power: Float,
        segments: &[SegmentFlowConditions],
        density: Float,
        direction: Float,
    ) -> Float {
        let power = internal_flow.flow_power_from_fan_power(fan_power);

        if power < MIN_POSITIVE {
            return 0.0;
        }

        // Initial guess when neglecting the external pressure:
        // P = s * phi * A * sqrt(2 * s / (density * K)) => s = (P / (phi * A * sqrt(2 / (density * K))))^(2/3)
        let total_area: Float = segments.iter().map(|segment| segment.area).sum();

        let flow_factor = internal_flow.porous_fraction * total_area * (
            2.0 / (density * internal_flow.loss_coefficient)
        ).sqrt();

        if flow_factor < MIN_POSITIVE {
            return 0.0;
        }

        // The internal suction where the first segment starts to get inflow. Below this value,
        // there is no flow, and the power is zero independent of the suction.
        let threshold_suction = segments.iter()
            .filter(|segment| segment.velocity > MIN_POSITIVE && segment.area > MIN_POSITIVE)
            .map(|segment| {
                let lift_coefficient = direction * self.lift_coefficient(
                    segment.angle_of_attack, 0.0
                );

                -internal_flow.external_pressure(lift_coefficient, density, segment.velocity)
            })
            .fold(Float::INFINITY, Float::min);

        if !threshold_suction.is_finite() {
            return 0.0;
        }

        let threshold_suction = threshold_suction.max(0.0);

        // The suction is written as s = s_threshold + x^2 so that the solver never ends up in the
        // region with zero flow, where the residual is constant and the secant method fails.
        let internal_suction = |x: Float| threshold_suction + x * x;

        let first_guess = (power / flow_factor).powf(2.0 / 3.0).sqrt();
        let second_guess = 1.1 * first_guess;

        let x = secant_solver(
            |x| {
                let internal_suction = internal_suction(x);

                internal_suction * self.total_volume_flow(
                    internal_flow, segments, density, internal_suction, direction
                ) / power - 1.0
            },
            first_guess,
            second_guess,
            &internal_flow.solver_settings.power_solver_settings()
        );

        internal_suction(x)
    }

    /// Returns the suction coefficient on all segments, for a given fan input power.
    ///
    /// The absolute value of the power is used to compute the suction, while the sign of the power
    /// sets the direction, which is applied to the returned suction coefficients. A power of zero
    /// gives zero suction coefficient on all segments.
    pub fn suction_coefficients_from_power(
        &self,
        internal_flow: &InternalFlow,
        power: Float,
        segments: &[SegmentFlowConditions],
        density: Float
    ) -> Vec<Float> {
        let power_abs = power.abs();

        if power_abs < MIN_POSITIVE {
            return vec![0.0; segments.len()];
        }

        let direction = power.signum();

        let internal_suction = self.internal_suction_from_power(
            internal_flow, power_abs, segments, density, direction
        );

        self.suction_coefficients(internal_flow, segments, density, internal_suction, direction)
            .into_iter()
            .map(|suction_coefficient| direction * suction_coefficient)
            .collect()
    }
}
