use stormath::type_aliases::Float;
use stormath::consts::MIN_POSITIVE;
use stormath::solvers::{secant_solver, brent_solver, SolverSettings};

use serde::{Serialize, Deserialize};

use super::SegmentFlowConditions;
use super::fan::Fan;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for the numerical solvers used when computing the internal suction of a wing, or of an
/// internal compartment, from the internal state of the wing
pub struct InternalFlowSolverSettings {
    #[serde(default="InternalFlowSolverSettings::default_wing_tolerance")]
    /// Relative tolerance when solving for the internal suction of the whole wing, either from the
    /// power or from the balance between the fan curves and the flow through the wing.
    pub wing_tolerance: Float,
    #[serde(default="InternalFlowSolverSettings::default_wing_max_iterations")]
    pub wing_max_iterations: usize,
}

impl InternalFlowSolverSettings {
    fn default_wing_tolerance() -> Float {1e-6}
    fn default_wing_max_iterations() -> usize {50}

    pub fn wing_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            tolerance: self.wing_tolerance,
            max_iterations: self.wing_max_iterations,
        }
    }
}

impl Default for InternalFlowSolverSettings {
    fn default() -> Self {
        Self {
            wing_tolerance: Self::default_wing_tolerance(),
            wing_max_iterations: Self::default_wing_max_iterations(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "InternalFlowInput")]
/// Model for estimating the flow through the porous surface of a wing, where all segments in the
/// same internal compartment share the same internal volume. The model does not include the fan,
/// which is part of the internal state conversion in the active air flow foil.
///
/// The external pressure on the porous surface is modeled as a linear function of the suction
/// coefficient: C_p,ext = cp_no_suction + cp_suction_gradient * C_Q. It does not depend on the
/// angle of attack or the lift. The suction coefficient on each segment can therefore be computed
/// directly from the internal suction, without involving the foil model.
///
/// The parameters are checked when the model is deserialized, or created with
/// [`new`](InternalFlow::new).
pub struct InternalFlow {
    /// External pressure coefficient on the porous surface without suction
    pub cp_no_suction: Float,
    /// Change in the external pressure coefficient per unit suction coefficient. Must be zero or
    /// negative, i.e., more suction can not increase the external pressure.
    pub cp_suction_gradient: Float,
    /// Loss coefficient for the flow through the porous surface. Must be larger than zero.
    pub loss_coefficient: Float,
    /// The fraction of the segment area that is porous. Must be larger than zero.
    pub porous_fraction: Float,
    pub solver_settings: InternalFlowSolverSettings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Unchecked version of the internal flow model, used for deserialization
struct InternalFlowInput {
    cp_no_suction: Float,
    cp_suction_gradient: Float,
    loss_coefficient: Float,
    porous_fraction: Float,
    #[serde(default)]
    solver_settings: InternalFlowSolverSettings,
}

impl TryFrom<InternalFlowInput> for InternalFlow {
    type Error = String;

    fn try_from(input: InternalFlowInput) -> Result<Self, Self::Error> {
        Self::new(
            input.cp_no_suction,
            input.cp_suction_gradient,
            input.loss_coefficient,
            input.porous_fraction,
            input.solver_settings
        )
    }
}

impl InternalFlow {
    /// Creates a new internal flow model, and checks that the parameters are valid
    pub fn new(
        cp_no_suction: Float,
        cp_suction_gradient: Float,
        loss_coefficient: Float,
        porous_fraction: Float,
        solver_settings: InternalFlowSolverSettings
    ) -> Result<Self, String> {
        if cp_suction_gradient > 0.0 {
            return Err(format!(
                "The cp_suction_gradient in the internal flow model must be zero or negative, as \
                more suction can not increase the external pressure. Value: {}",
                cp_suction_gradient
            ));
        }

        if loss_coefficient <= 0.0 {
            return Err(format!(
                "The loss_coefficient in the internal flow model must be larger than zero. \
                Value: {}",
                loss_coefficient
            ));
        }

        if porous_fraction <= 0.0 {
            return Err(format!(
                "The porous_fraction in the internal flow model must be larger than zero. \
                Value: {}",
                porous_fraction
            ));
        }

        Ok(Self {
            cp_no_suction,
            cp_suction_gradient,
            loss_coefficient,
            porous_fraction,
            solver_settings,
        })
    }

    /// Computes the external pressure coefficient on the porous surface, as a function of the
    /// magnitude of the suction coefficient
    pub fn external_pressure_coefficient(&self, suction_coefficient: Float) -> Float {
        self.cp_no_suction + self.cp_suction_gradient * suction_coefficient.abs()
    }

    /// The factor between the pressure drop coefficient over the porous surface and the suction
    /// coefficient squared: dp / q = (K / phi^2) * C_Q^2
    fn loss_factor(&self) -> Float {
        self.loss_coefficient / self.porous_fraction.powi(2)
    }

    /// Computes the non-dimensional internal suction, s / q, where q = 0.5 * density * U^2, that
    /// gives a specified suction coefficient. This is the inverse of
    /// [`suction_coefficient`](InternalFlow::suction_coefficient). Only the magnitude of the
    /// suction coefficient is used.
    pub fn internal_suction_coefficient(&self, suction_coefficient: Float) -> Float {
        let suction_coefficient = suction_coefficient.abs();

        self.loss_factor() * suction_coefficient.powi(2) -
            self.external_pressure_coefficient(suction_coefficient)
    }

    /// Computes the magnitude of the suction coefficient, C_Q = Q / (U * A), for a given
    /// non-dimensional internal suction, s / q.
    ///
    /// The pressure balance over the porous surface gives a quadratic equation in C_Q:
    ///
    /// (K / phi^2) * C_Q^2 - cp_suction_gradient * C_Q - (cp_no_suction + s / q) = 0
    ///
    /// Since the gradient is not positive, there is exactly one positive solution when
    /// cp_no_suction + s / q is positive, and no flow into the foil otherwise. Outflow is not
    /// modeled, so the returned value is never negative.
    pub fn suction_coefficient(&self, internal_suction_coefficient: Float) -> Float {
        let constant_term = self.cp_no_suction + internal_suction_coefficient;

        if constant_term <= 0.0 {
            return 0.0;
        }

        let a = self.loss_factor();
        let b = -self.cp_suction_gradient;

        // Numerically stable form of the positive root of a * x^2 + b * x - c = 0
        2.0 * constant_term / (b + (b * b + 4.0 * a * constant_term).sqrt())
    }

    /// Computes the magnitude of the suction coefficient on a single segment, for a given internal
    /// suction, in Pa. Returns zero if the velocity or the area of the segment is zero.
    pub fn segment_suction_coefficient(
        &self,
        segment: &SegmentFlowConditions,
        density: Float,
        internal_suction: Float,
    ) -> Float {
        if segment.velocity < MIN_POSITIVE || segment.area < MIN_POSITIVE {
            return 0.0;
        }

        let dynamic_pressure = 0.5 * density * segment.velocity.powi(2);

        self.suction_coefficient(internal_suction / dynamic_pressure)
    }

    /// Returns the magnitude of the suction coefficient on all segments, for a given internal
    /// suction
    pub fn suction_coefficients(
        &self,
        segments: &[SegmentFlowConditions],
        density: Float,
        internal_suction: Float,
    ) -> Vec<Float> {
        segments.iter().map(|segment| {
            self.segment_suction_coefficient(segment, density, internal_suction)
        }).collect()
    }

    /// Returns the total volume flow into the wing, summed over all segments that are not
    /// virtual, for a given internal suction
    pub fn total_volume_flow(
        &self,
        segments: &[SegmentFlowConditions],
        density: Float,
        internal_suction: Float,
    ) -> Float {
        segments.iter().filter(|segment| !segment.is_virtual).map(|segment| {
            self.segment_suction_coefficient(segment, density, internal_suction) *
                segment.velocity * segment.area
        }).sum()
    }

    /// The internal suction where the first segment starts to get inflow, i.e., where the internal
    /// suction balances the external pressure without suction. Below this value, there is no flow
    /// into the wing. Returns `None` if no segment can get any flow, i.e., if all segments have
    /// zero velocity or area.
    pub fn threshold_internal_suction(
        &self,
        segments: &[SegmentFlowConditions],
        density: Float,
    ) -> Option<Float> {
        let threshold_suction = segments.iter()
            .filter(|segment| {
                !segment.is_virtual && segment.velocity > MIN_POSITIVE && segment.area > MIN_POSITIVE
            })
            .map(|segment| {
                -self.cp_no_suction * 0.5 * density * segment.velocity.powi(2)
            })
            .fold(Float::INFINITY, Float::min);

        if threshold_suction.is_finite() {
            Some(threshold_suction.max(0.0))
        } else {
            None
        }
    }

    /// Solves for the internal suction that gives the specified power delivered to the flow.
    ///
    /// All segments are assumed to share the same internal volume, so that the power delivered to
    /// the flow is computed from the total volume flow: P_flow = s * sum(Q). Returns zero if the
    /// power is zero or negative.
    pub fn internal_suction_from_flow_power(
        &self,
        flow_power: Float,
        segments: &[SegmentFlowConditions],
        density: Float,
    ) -> Float {
        if flow_power < MIN_POSITIVE {
            return 0.0;
        }

        // Initial guess when neglecting the external pressure:
        // P = s * phi * A * sqrt(2 * s / (density * K)) => s = (P / (phi * A * sqrt(2 / (density * K))))^(2/3)
        let total_area: Float = segments.iter()
            .filter(|segment| !segment.is_virtual)
            .map(|segment| segment.area)
            .sum();

        let flow_factor = self.porous_fraction * total_area * (
            2.0 / (density * self.loss_coefficient)
        ).sqrt();

        if flow_factor < MIN_POSITIVE {
            return 0.0;
        }

        let threshold_suction = match self.threshold_internal_suction(segments, density) {
            Some(threshold_suction) => threshold_suction,
            None => return 0.0,
        };

        // The suction is written as s = s_threshold + x^2 so that the solver never ends up in the
        // region with zero flow, where the residual is constant and the secant method fails.
        let internal_suction = |x: Float| threshold_suction + x * x;

        let first_guess = (flow_power / flow_factor).powf(2.0 / 3.0).sqrt();
        let second_guess = 1.1 * first_guess;

        let x = secant_solver(
            |x| {
                let internal_suction = internal_suction(x);

                internal_suction * self.total_volume_flow(
                    segments, density, internal_suction
                ) / flow_power - 1.0
            },
            first_guess,
            second_guess,
            &self.solver_settings.wing_solver_settings()
        );

        internal_suction(x)
    }

    /// Solves for the internal suction at the operating point of the fans, for a given rotational
    /// speed.
    ///
    /// The operating point is where the pressure rise over the fans balances the internal suction
    /// and the duct losses, at the total volume flow through the wing:
    /// dp_fan(Q(s), n) = s + dp_duct(Q(s)).
    ///
    /// The internal suction is always between the threshold where the flow into the wing starts
    /// and the maximum pressure rise of the fans, which is used as a bracket for the solver. If
    /// the fans can not overcome the threshold, there is no flow, and the threshold is returned.
    pub fn internal_suction_from_fan_speed(
        &self,
        fan: &Fan,
        rotational_speed: Float,
        segments: &[SegmentFlowConditions],
        density: Float,
    ) -> Float {
        if rotational_speed < MIN_POSITIVE {
            return 0.0;
        }

        let threshold_suction = match self.threshold_internal_suction(segments, density) {
            Some(threshold_suction) => threshold_suction,
            None => return 0.0,
        };

        let max_pressure_rise = fan.max_pressure_rise(rotational_speed, density);

        if max_pressure_rise <= threshold_suction {
            return threshold_suction;
        }

        let residual = |internal_suction: Float| {
            let total_volume_flow = self.total_volume_flow(segments, density, internal_suction);

            (
                fan.pressure_rise(total_volume_flow, rotational_speed, density) -
                internal_suction -
                fan.duct_loss(total_volume_flow)
            ) / max_pressure_rise
        };

        brent_solver(
            residual,
            threshold_suction,
            max_pressure_rise,
            &self.solver_settings.wing_solver_settings()
        ).unwrap_or(threshold_suction)
    }
}
