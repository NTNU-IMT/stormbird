
use serde::{Serialize, Deserialize};
use stormath::type_aliases::Float;
use stormath::consts::MIN_POSITIVE;

use super::varying_foil::VaryingFoil;
use super::integrate_power_coefficient;
use crate::line_force_model::wing_internal_state::{WingInternalState, InternalCompartments};

pub use super::SegmentFlowConditions;

pub mod internal_flow;
pub mod fan;

use internal_flow::InternalFlow;
use fan::Fan;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines how the internal state set on the wing is converted to the internal state used by the
/// foil model on each segment.
///
/// **Note**: the meaning of the `internal_state_data` in the varying foil model depends on the
/// variant, as described below.
///
/// For all variants, the sign of the wing state sets the direction of the suction/flap, and is
/// preserved in the section internal states. The magnitude of the power, rotational speed or
/// suction coefficient is therefore given by the absolute value of the wing state.
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
    /// internal flow model and a constant fan efficiency. The foil data must be given as a
    /// function of C_Q.
    PowerToInternalFlow {
        internal_flow: InternalFlow,
        #[serde(default = "InternalStateConversion::default_fan_efficiency")]
        /// Efficiency of the fan, defined as the ratio between the power delivered to the flow,
        /// P_flow = s * Q, and the input power to the fan. Must be larger than zero.
        fan_efficiency: Float,
    },
    /// The wing state is the rotational speed of the fans, in revolutions per second, which is
    /// converted to a suction coefficient on each segment by solving for the operating point
    /// where the pressure rise over the fans balances the internal suction and the duct losses.
    /// The power is given by the fan curves. The foil data must be given as a function of C_Q.
    FanSpeedToInternalFlow {
        internal_flow: InternalFlow,
        fan: Fan,
    },
}

impl InternalStateConversion {
    pub fn default_fan_efficiency() -> Float {1.0}
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

impl ActiveAirFlowFoil {
    pub fn new_from_string(input_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input_str)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

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


    /// Computes the power coefficient, c_P = P / (0.5 * density * A * U^3), from the internal
    /// state of the foil model. The power is always positive, as the sign of the internal state
    /// only determines the direction of the suction/flap.
    ///
    /// The power coefficient only depends on the internal state, as the internal flow model does
    /// not depend on the angle of attack.
    ///
    /// For the `FanSpeedToInternalFlow` conversion, the efficiency of the fans depends on the
    /// operating point of the whole wing, and can not be computed for a single segment. The
    /// returned value is then the power delivered to the flow, without the losses in the fans.
    /// Use [`wing_power`](ActiveAirFlowFoil::wing_power) to get the power for the whole wing.
    pub fn power_coefficient(&self, internal_state: Float) -> Float {
        match &self.internal_state_conversion {
            InternalStateConversion::SuctionCoefficientDirect => 0.0,
            InternalStateConversion::PowerToPowerCoefficient => internal_state.abs(),
            InternalStateConversion::PowerToInternalFlow { internal_flow, fan_efficiency } => {
                Self::flow_power_coefficient(internal_flow, internal_state) / fan_efficiency
            },
            InternalStateConversion::FanSpeedToInternalFlow { internal_flow, .. } => {
                Self::flow_power_coefficient(internal_flow, internal_state)
            },
        }
    }

    /// Computes the coefficient for the power delivered to the flow, P_flow = s * Q, from the
    /// suction coefficient, by inverting the internal flow model.
    fn flow_power_coefficient(internal_flow: &InternalFlow, internal_state: Float) -> Float {
        let suction_coefficient = internal_state.abs();

        if suction_coefficient < MIN_POSITIVE {
            return 0.0;
        }

        // P_flow = s * Q = (s / q) * C_Q * 0.5 * density * A * U^3
        suction_coefficient * internal_flow.internal_suction_coefficient(suction_coefficient)
    }

    /// Returns true if the internal state conversion uses an internal flow model, and therefore
    /// can have multiple internal compartments.
    pub fn uses_internal_compartments(&self) -> bool {
        matches!(
            self.internal_state_conversion,
            InternalStateConversion::PowerToInternalFlow { .. } |
            InternalStateConversion::FanSpeedToInternalFlow { .. }
        )
    }

    /// Computes the power needed to operate the wing.
    ///
    /// For the `FanSpeedToInternalFlow` conversion, the power is given by the fan curves in each
    /// compartment, using the rotational speed for the compartment and the volume flow through it.
    /// For the other conversions, the power coefficient is integrated over the wing.
    pub fn wing_power(
        &self,
        wing_state: &WingInternalState,
        segments: &[SegmentFlowConditions],
        compartments: &InternalCompartments,
        section_states: &[Float],
        density: Float
    ) -> Float {
        match &self.internal_state_conversion {
            InternalStateConversion::FanSpeedToInternalFlow { fan, .. } => {
                (0..compartments.nr_compartments()).map(|compartment_index| {
                    let volume_flow: Float = segments.iter().zip(section_states)
                        .filter(|(segment, _)| {
                            !segment.is_virtual && segment.compartment_index == compartment_index
                        })
                        .map(|(segment, &section_state)| {
                            section_state.abs() * segment.velocity * segment.area
                        }).sum();

                    let rotational_speed = Self::compartment_rotational_speed(
                        wing_state, compartments, compartment_index
                    ).abs();

                    fan.power(volume_flow, rotational_speed, density)
                }).sum()
            },
            _ => integrate_power_coefficient(
                segments, section_states, density, |section_state| {
                    self.power_coefficient(section_state)
                }
            ),
        }
    }

    /// Converts the internal state set on the wing to the internal state of the foil model on each
    /// segment, according to the internal state conversion.
    ///
    /// The sign of the wing state is preserved in the section internal states, so that it can be
    /// used to set the direction of the suction/flap.
    ///
    /// For a span varying wing state, the power based conversions interpret the values as power
    /// per span length. Virtual segments get the same type of internal state as the rest of the
    /// wing, but are not included in the power or in the flow through the wing.
    ///
    /// For the conversions with an internal flow model, each internal compartment is solved
    /// separately, with its own internal pressure. A constant power is divided equally between the
    /// compartments, while a power per span length is integrated over each compartment. The
    /// rotational speed of the fans in each compartment is the mean value over the compartment.
    pub fn section_internal_states(
        &self,
        wing_state: &WingInternalState,
        segments: &[SegmentFlowConditions],
        compartments: &InternalCompartments,
        density: Float
    ) -> Vec<Float> {
        match &self.internal_state_conversion {
            InternalStateConversion::SuctionCoefficientDirect => {
                segments.iter().map(|segment| {
                    wing_state.value_at(segment.non_dim_span_location)
                }).collect()
            },
            InternalStateConversion::PowerToPowerCoefficient => {
                Self::power_coefficients_from_power(wing_state, segments, density)
            },
            InternalStateConversion::PowerToInternalFlow { internal_flow, fan_efficiency } => {
                Self::solve_per_compartment(
                    segments,
                    compartments,
                    |compartment_segments, compartment_index| {
                        let power = Self::compartment_power(
                            wing_state, segments, compartments, compartment_index
                        );

                        Self::suction_coefficients_from_power(
                            internal_flow, *fan_efficiency, power, compartment_segments, density
                        )
                    }
                )
            },
            InternalStateConversion::FanSpeedToInternalFlow { internal_flow, fan } => {
                Self::solve_per_compartment(
                    segments,
                    compartments,
                    |compartment_segments, compartment_index| {
                        let rotational_speed = Self::compartment_rotational_speed(
                            wing_state, compartments, compartment_index
                        );

                        Self::suction_coefficients_from_fan_speed(
                            internal_flow, fan, rotational_speed, compartment_segments, density
                        )
                    }
                )
            },
        }
    }

    /// Computes the section internal states for each compartment separately, using the segments
    /// that belong to the compartment, and collects the result in the original order.
    fn solve_per_compartment(
        segments: &[SegmentFlowConditions],
        compartments: &InternalCompartments,
        solve: impl Fn(&[SegmentFlowConditions], usize) -> Vec<Float>,
    ) -> Vec<Float> {
        let mut section_states = vec![0.0; segments.len()];

        for compartment_index in 0..compartments.nr_compartments() {
            let indices: Vec<usize> = segments.iter().enumerate()
                .filter(|(_, segment)| segment.compartment_index == compartment_index)
                .map(|(index, _)| index)
                .collect();

            if indices.is_empty() {
                continue;
            }

            let compartment_segments: Vec<SegmentFlowConditions> = indices.iter()
                .map(|&index| segments[index].clone())
                .collect();

            let compartment_states = solve(&compartment_segments, compartment_index);

            for (&index, state) in indices.iter().zip(compartment_states) {
                section_states[index] = state;
            }
        }

        section_states
    }

    /// The power for a compartment, for a wing state that represents power. A constant value is
    /// the total power for the wing, which is divided equally between the compartments. A span
    /// varying value is the power per span length, which is integrated over the compartment.
    fn compartment_power(
        wing_state: &WingInternalState,
        segments: &[SegmentFlowConditions],
        compartments: &InternalCompartments,
        compartment_index: usize
    ) -> Float {
        match wing_state {
            WingInternalState::Constant(power) => *power / compartments.nr_compartments() as Float,
            WingInternalState::SpanVarying(_) => {
                let span_length: Float = segments.iter().map(|segment| segment.span_length).sum();

                let (start, end) = compartments.range(compartment_index);

                wing_state.span_integral(start, end, span_length)
            },
        }
    }

    /// The rotational speed of the fans in a compartment, which is the mean value of the wing state
    /// over the compartment
    fn compartment_rotational_speed(
        wing_state: &WingInternalState,
        compartments: &InternalCompartments,
        compartment_index: usize
    ) -> Float {
        let (start, end) = compartments.range(compartment_index);

        wing_state.mean(start, end)
    }

    /// Converts a wing state that represents power to a power coefficient on each segment.
    fn power_coefficients_from_power(
        wing_state: &WingInternalState,
        segments: &[SegmentFlowConditions],
        density: Float
    ) -> Vec<Float> {
        match wing_state {
            WingInternalState::Constant(power) => {
                // The power is distributed to each segment proportional to the segment area:
                // P_segment = P * A_segment / A_wing. The segment power coefficient is then
                // P_segment / (0.5 * A_segment * density * U^3), where the segment area cancels,
                // giving P / (0.5 * A_wing * density * U^3).
                let wing_area: Float = segments.iter()
                    .filter(|segment| !segment.is_virtual)
                    .map(|segment| segment.area)
                    .sum();

                segments.iter().map(|segment| {
                    if wing_area > MIN_POSITIVE && segment.velocity > MIN_POSITIVE {
                        power / (0.5 * wing_area * density * segment.velocity.powi(3))
                    } else {
                        0.0
                    }
                }).collect()
            },
            WingInternalState::SpanVarying(_) => {
                // The power per span length gives the power on each segment as
                // P_segment = power_per_span * L_segment. The power coefficient is then
                // power_per_span / (0.5 * chord * density * U^3)
                segments.iter().map(|segment| {
                    let chord_length = segment.chord_length();

                    if chord_length > MIN_POSITIVE && segment.velocity > MIN_POSITIVE {
                        let power_per_span = wing_state.value_at(segment.non_dim_span_location);

                        power_per_span / (0.5 * chord_length * density * segment.velocity.powi(3))
                    } else {
                        0.0
                    }
                }).collect()
            },
        }
    }

    /// Estimates the wing state that gives a specified section internal state. This is the inverse
    /// of [`section_internal_states`](ActiveAirFlowFoil::section_internal_states), assuming the
    /// section internal state is uniform over the wing, and that the flow conditions can be
    /// represented by a single velocity.
    ///
    /// For the power based conversions, the power is calculated from the power coefficient of the
    /// section model: P = sign(state) * c_P(state) * 0.5 * density * A_wing * U^3. For the
    /// fan based conversion, the rotational speed is found from the fan curves, at the operating
    /// point given by the suction coefficient. With several internal compartments, the area of the
    /// wing is assumed to be divided equally between them.
    pub fn wing_state_from_section_state(
        &self,
        section_state: Float,
        velocity: Float,
        wing_area: Float,
        nr_compartments: usize,
        density: Float
    ) -> Float {
        match &self.internal_state_conversion {
            InternalStateConversion::SuctionCoefficientDirect => section_state,
            InternalStateConversion::PowerToPowerCoefficient |
            InternalStateConversion::PowerToInternalFlow { .. } => {
                let power_coefficient = self.power_coefficient(section_state);

                section_state.signum() * power_coefficient *
                    0.5 * density * wing_area * velocity.abs().powi(3)
            },
            InternalStateConversion::FanSpeedToInternalFlow { internal_flow, fan } => {
                let suction_coefficient = section_state.abs();
                let velocity = velocity.abs();

                if suction_coefficient < MIN_POSITIVE || velocity < MIN_POSITIVE {
                    return 0.0;
                }

                let direction = section_state.signum();

                let internal_suction = internal_flow.internal_suction_coefficient(
                    suction_coefficient
                ) * 0.5 * density * velocity.powi(2);

                let compartment_area = wing_area / nr_compartments.max(1) as Float;

                let total_volume_flow = suction_coefficient * velocity * compartment_area;

                let required_pressure_rise = internal_suction + fan.duct_loss(total_volume_flow);

                direction * fan.rotational_speed_for_operating_point(
                    total_volume_flow,
                    required_pressure_rise,
                    density,
                    &internal_flow.solver_settings.wing_solver_settings()
                )
            },
        }
    }

    /// Returns the suction coefficient on all segments, for a given fan input power and a constant
    /// fan efficiency.
    ///
    /// The absolute value of the power is used to compute the suction, while the sign of the power
    /// sets the direction, which is applied to the returned suction coefficients. A power of zero
    /// gives zero suction coefficient on all segments.
    pub fn suction_coefficients_from_power(
        internal_flow: &InternalFlow,
        fan_efficiency: Float,
        power: Float,
        segments: &[SegmentFlowConditions],
        density: Float
    ) -> Vec<Float> {
        let power_abs = power.abs();

        if power_abs < MIN_POSITIVE {
            return vec![0.0; segments.len()];
        }

        let direction = power.signum();

        let internal_suction = internal_flow.internal_suction_from_flow_power(
            fan_efficiency * power_abs, segments, density
        );

        internal_flow.suction_coefficients(segments, density, internal_suction)
            .into_iter()
            .map(|suction_coefficient| direction * suction_coefficient)
            .collect()
    }

    /// Returns the suction coefficient on all segments, for a given rotational speed of the fans.
    ///
    /// The absolute value of the rotational speed is used to compute the suction, while the sign
    /// sets the direction, which is applied to the returned suction coefficients. A rotational
    /// speed of zero gives zero suction coefficient on all segments.
    pub fn suction_coefficients_from_fan_speed(
        internal_flow: &InternalFlow,
        fan: &Fan,
        rotational_speed: Float,
        segments: &[SegmentFlowConditions],
        density: Float
    ) -> Vec<Float> {
        let rotational_speed_abs = rotational_speed.abs();

        if rotational_speed_abs < MIN_POSITIVE {
            return vec![0.0; segments.len()];
        }

        let direction = rotational_speed.signum();

        let internal_suction = internal_flow.internal_suction_from_fan_speed(
            fan, rotational_speed_abs, segments, density
        );

        internal_flow.suction_coefficients(segments, density, internal_suction)
            .into_iter()
            .map(|suction_coefficient| direction * suction_coefficient)
            .collect()
    }
}
