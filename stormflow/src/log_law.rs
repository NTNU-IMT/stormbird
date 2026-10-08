//! The standard log-law of the wall, shared by the wall model of the velocity solver and the wall
//! functions of the turbulence models.

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Copy)]
/// The constants of the standard log-law wall functions, with the same default values as in
/// OpenFOAM. `c_mu` is only used by the wall functions of the turbulence models.
pub struct WallFunctionConstants {
    pub c_mu: Float,
    pub kappa: Float,
    pub e: Float,
}

impl Default for WallFunctionConstants {
    fn default() -> Self {
        Self {
            c_mu: 0.09,
            kappa: 0.41,
            e: 9.8,
        }
    }
}

/// The number of fixed point iterations used to solve the log-law for the friction velocity
pub const NR_FRICTION_VELOCITY_ITERATIONS: usize = 10;

impl WallFunctionConstants {
    /// The y+ value where the viscous sublayer and the log-law region intersect, computed in the
    /// same way as in OpenFOAM's `nutWallFunction`.
    pub fn y_plus_lam(&self) -> Float {
        let mut y_plus_lam: Float = 11.0;

        for _ in 0..10 {
            y_plus_lam = (self.e * y_plus_lam).max(1.0).ln() / self.kappa;
        }

        y_plus_lam
    }

    #[inline(always)]
    /// The friction velocity, `u_tau = sqrt(tau_wall / rho)`, for a tangential `velocity` at
    /// `distance` from the wall. Uses the linear profile of the viscous sublayer, `u+ = y+`, when
    /// that gives `y+ < y_plus_lam`, and otherwise the log-law, `u+ = ln(E y+) / kappa`, which is
    /// solved with fixed point iterations, starting from the sublayer value.
    pub fn friction_velocity(
        &self,
        velocity: Float,
        distance: Float,
        viscosity: Float,
        y_plus_lam: Float
    ) -> Float {
        if velocity <= 0.0 {
            return 0.0;
        }

        let laminar_friction_velocity = (viscosity * velocity / distance).sqrt();

        if laminar_friction_velocity * distance / viscosity <= y_plus_lam {
            return laminar_friction_velocity;
        }

        let mut friction_velocity = laminar_friction_velocity;

        for _ in 0..NR_FRICTION_VELOCITY_ITERATIONS {
            let log_term = (self.e * distance * friction_velocity / viscosity).max(1.0 + 1e-3).ln();

            friction_velocity = self.kappa * velocity / log_term;
        }

        friction_velocity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y_plus_lam_matches_openfoam() {
        let y_plus_lam = WallFunctionConstants::default().y_plus_lam();

        assert!((y_plus_lam - 11.53).abs() < 0.01, "{y_plus_lam}");
    }

    #[test]
    fn friction_velocity_satisfies_the_log_law() {
        let constants = WallFunctionConstants::default();
        let y_plus_lam = constants.y_plus_lam();

        let (velocity, distance, viscosity) = (10.0, 1.0, 1.5e-5);

        let u_tau = constants.friction_velocity(velocity, distance, viscosity, y_plus_lam);

        let log_law_velocity = u_tau / constants.kappa * (constants.e * distance * u_tau / viscosity).ln();

        assert!((log_law_velocity - velocity).abs() < 1e-3 * velocity, "{u_tau}: {log_law_velocity}");
    }

    #[test]
    fn friction_velocity_in_the_viscous_sublayer() {
        let constants = WallFunctionConstants::default();
        let y_plus_lam = constants.y_plus_lam();

        let (velocity, distance, viscosity) = (0.01, 0.001, 1.5e-5);

        let u_tau = constants.friction_velocity(velocity, distance, viscosity, y_plus_lam);

        assert!((u_tau * u_tau - viscosity * velocity / distance).abs() < 1e-9);
    }
}
