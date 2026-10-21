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
    #[serde(default="InternalFlowSolverSettings::default_power_tolerance")]
    /// Tolerance on the power, relative to the target power, when solving for the internal suction
    pub power_tolerance: Float,
    #[serde(default="InternalFlowSolverSettings::default_max_iterations")]
    pub power_max_iterations: usize,
}

impl InternalFlowSolverSettings {
    fn default_segment_tolerance() -> Float {1e-8}
    fn default_power_tolerance() -> Float {1e-6}
    fn default_max_iterations() -> usize {20}

    pub fn segment_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            tolerance: self.segment_tolerance,
            max_iterations: self.segment_max_iterations,
        }
    }

    pub fn power_solver_settings(&self) -> SolverSettings {
        SolverSettings {
            tolerance: self.power_tolerance,
            max_iterations: self.power_max_iterations,
        }
    }
}

impl Default for InternalFlowSolverSettings {
    fn default() -> Self {
        Self {
            segment_tolerance: Self::default_segment_tolerance(),
            segment_max_iterations: Self::default_max_iterations(),
            power_tolerance: Self::default_power_tolerance(),
            power_max_iterations: Self::default_max_iterations(),
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
/// Model for estimating the internal flow properties on a line segment
pub struct InternalFlow {
    pub cp_zero_cl: Float,
    pub cp_gradient: Float,
    pub loss_coefficient: Float,
    pub porous_fraction: Float,
    #[serde(default="InternalFlow::default_fan_efficiency")]
    /// Efficiency of the fan, defined as the ratio between the power delivered to the flow,
    /// P_flow = s * Q, and the input power to the fan. Must be larger than zero.
    pub fan_efficiency: Float,
    #[serde(default)]
    pub solver_settings: InternalFlowSolverSettings,
}

impl InternalFlow {
    pub fn default_fan_efficiency() -> Float {1.0}

    /// Converts the input power to the fan to the power delivered to the flow
    pub fn flow_power_from_fan_power(&self, fan_power: Float) -> Float {
        fan_power * self.fan_efficiency
    }

    /// Converts the power delivered to the flow to the input power to the fan
    pub fn fan_power_from_flow_power(&self, flow_power: Float) -> Float {
        flow_power / self.fan_efficiency
    }

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

    /// Computes the fan input power associated with the flow through the segment:
    /// P = Q * s / fan_efficiency.
    ///
    /// The value is negative if the flow goes out of the foil while the internal suction is
    /// positive. Only the sum over all segments sharing the same internal volume is physically
    /// meaningful, as this is the power the fan must deliver.
    pub fn segment_power(&self, input: &InternalFlowInput) -> Float {
        self.fan_power_from_flow_power(self.segment_volume_flow(input) * input.internal_suction)
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