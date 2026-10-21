use stormath::type_aliases::Float;
use stormath::consts::MIN_POSITIVE;
use stormath::solvers::SolverSettings;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Settings for the numerical solvers used when computing the internal state from the internal
/// flow model
pub struct InternalFlowSolverSettings {
    #[serde(default="InternalFlowSolverSettings::default_segment_tolerance")]
    /// Absolute tolerance on the suction coefficient, when solving for a consistent suction
    /// coefficient on a single segment
    pub segment_tolerance: Float,
    #[serde(default="InternalFlowSolverSettings::default_max_iterations")]
    pub segment_max_iterations: usize,
    #[serde(default="InternalFlowSolverSettings::default_wing_tolerance")]
    /// Relative tolerance when solving for the internal suction of the whole wing, either from the
    /// power or from the balance between the fan curves and the flow through the wing.
    pub wing_tolerance: Float,
    #[serde(default="InternalFlowSolverSettings::default_wing_max_iterations")]
    pub wing_max_iterations: usize,
}

impl InternalFlowSolverSettings {
    fn default_segment_tolerance() -> Float {1e-8}
    fn default_wing_tolerance() -> Float {1e-6}
    fn default_max_iterations() -> usize {20}
    fn default_wing_max_iterations() -> usize {50}

    pub fn segment_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            tolerance: self.segment_tolerance,
            max_iterations: self.segment_max_iterations,
        }
    }

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
            segment_tolerance: Self::default_segment_tolerance(),
            segment_max_iterations: Self::default_max_iterations(),
            wing_tolerance: Self::default_wing_tolerance(),
            wing_max_iterations: Self::default_wing_max_iterations(),
        }
    }
}


#[derive(Debug, Clone)]
/// Structure used to group all required input to compute the internal flow.
///
/// The lift coefficient is defined as positive when the lift is directed towards the side with
/// the porous surface, i.e., the suction side.
pub struct InternalFlowInput {
    pub lift_coefficient: Float,
    pub segment_area: Float,
    pub density: Float,
    pub velocity: Float,
    pub internal_suction: Float
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Model for estimating the flow through the porous surface of a wing, where all segments share 
/// the same internal volume. The model does not include the fan, which is part of the internal 
/// state conversion in the active air flow foil.
pub struct InternalFlow {
    pub cp_zero_cl: Float,
    pub cp_gradient: Float,
    pub loss_coefficient: Float,
    pub porous_fraction: Float,
    #[serde(default)]
    pub solver_settings: InternalFlowSolverSettings,
}

impl InternalFlow {
    /// Computes the external pressure on the suction side of a foil, as a function of the lift
    /// coefficient
    pub fn external_pressure_coefficient(&self, cl: Float) -> Float {
        self.cp_zero_cl + self.cp_gradient * cl
    }

    pub fn external_pressure(&self, cl: Float, density: Float, velocity: Float) -> Float {
        0.5 * self.external_pressure_coefficient(cl) * density * velocity.powi(2)
    }

    /// Computes the volume flow into the segment through the porous surface.
    ///
    /// A positive value means flow into the foil (suction), while a negative value means flow out
    /// of the foil (blowing), which happens if the internal suction is not strong enough to
    /// overcome the external suction pressure.
    pub fn segment_volume_flow(&self, input: &InternalFlowInput) -> Float {
        let external_pressure = self.external_pressure(
            input.lift_coefficient,
            input.density,
            input.velocity
        );

        let pressure_drop = external_pressure + input.internal_suction;

        let porous_area = self.porous_fraction * input.segment_area;

        porous_area * pressure_drop.signum() * (
            2.0 * pressure_drop.abs() / (input.density * self.loss_coefficient)
        ).sqrt()
    }

    /// Computes the non-dimensional suction coefficient, defined as the volume flow divided by the
    /// velocity and the segment area: C_Q = Q / (U * A)
    ///
    /// Returns zero if the velocity or the segment area is zero, as the coefficient is undefined
    /// in that case.
    pub fn suction_coefficient(&self, input: &InternalFlowInput) -> Float {
        let normalization = input.velocity.abs() * input.segment_area;

        if normalization < MIN_POSITIVE {
            return 0.0;
        }

        self.segment_volume_flow(input) / normalization
    }

    /// Computes the power delivered to the flow through the segment: P_flow = Q * s. This does not 
    /// include losses in the fan.
    ///
    /// The value is negative if the flow goes out of the foil while the internal suction is
    /// positive. Only the sum over all segments sharing the same internal volume is physically
    /// meaningful.
    pub fn segment_flow_power(&self, input: &InternalFlowInput) -> Float {
        self.segment_volume_flow(input) * input.internal_suction
    }

    /// Computes the non-dimensional internal suction, s / q, where q = 0.5 * density * U^2, that
    /// gives a specified suction coefficient for a given lift coefficient. This is the inverse of
    /// [`suction_coefficient`](InternalFlow::suction_coefficient).
    pub fn internal_suction_coefficient(
        &self,
        suction_coefficient: Float,
        lift_coefficient: Float
    ) -> Float {
        if self.porous_fraction < MIN_POSITIVE {
            return 0.0;
        }

        // From C_Q = phi * sign(dp) * sqrt(|dp / q| / K)
        let pressure_drop_coefficient = self.loss_coefficient *
            suction_coefficient * suction_coefficient.abs() / self.porous_fraction.powi(2);

        pressure_drop_coefficient - self.external_pressure_coefficient(lift_coefficient)
    }
}