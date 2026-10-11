use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use stormbird::wind::environment::WindEnvironment;

use crate::grid::Grid;
use crate::geometry::WallGeometries;
use crate::velocity_solver::boundary_condisitions::VelocityBoundaryConditions;

use super::TurbulenceSolverSetup;
use super::models::TurbulenceModel;
use super::boundary_conditions::TurbulenceBoundaryConditions;
use super::transport::ConvectionScheme;
use super::wall_treatment::{
    WallTreatment,
    WallTreatmentEntries,
    WallFunctionConstants
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// The turbulence at the inlet, which is given through k and epsilon. Uses `C_mu = 0.09` and
/// `kappa = 0.41` independent of the model, as is common for inlet conditions.
pub enum InletTurbulence {
    /// `k = 1.5 (I |U|)^2` and `epsilon = C_mu^0.75 k^1.5 / L`, where `I` is the turbulence
    /// intensity, `|U|` the local inlet velocity, and `L` the turbulent length scale.
    IntensityAndLengthScale {
        turbulence_intensity: Float,
        length_scale: Float,
    },
    /// `k = 1.5 (I |U|)^2` and `epsilon = C_mu k^2 / (r nu)`, where `r` is the ratio between the
    /// eddy viscosity and the molecular viscosity, `nu`.
    IntensityAndViscosityRatio {
        turbulence_intensity: Float,
        eddy_viscosity_ratio: Float,
    },
    /// The equilibrium atmospheric boundary layer of Richards and Hoxey (1993), as used by
    /// OpenFOAM's `atmBoundaryLayerInlet` conditions: `k = u*^2 / sqrt(C_mu)` and
    /// `epsilon = u*^3 / (kappa (z + z0))`, where `u*` is the friction velocity, `z` the height
    /// above the water plane, and `z0` the roughness length.
    AtmosphericBoundaryLayer {
        friction_velocity: Float,
        roughness_length: Float,
    },
    /// The same values of k and epsilon at all heights
    Fixed {
        k: Float,
        epsilon: Float,
    },
}

impl Default for InletTurbulence {
    fn default() -> Self {
        Self::IntensityAndViscosityRatio {
            turbulence_intensity: 0.01,
            eddy_viscosity_ratio: 10.0,
        }
    }
}

impl InletTurbulence {
    /// Returns `(k, epsilon)` for the local inlet velocity magnitude and height
    pub fn k_and_epsilon(&self, velocity_magnitude: Float, height: Float, viscosity: Float) -> (Float, Float) {
        let constants = WallFunctionConstants::default();
        let c_mu = constants.c_mu;

        match *self {
            Self::IntensityAndLengthScale { turbulence_intensity, length_scale } => {
                let k = 1.5 * (turbulence_intensity * velocity_magnitude).powi(2);

                (k, c_mu.powf(0.75) * k.powf(1.5) / length_scale)
            },
            Self::IntensityAndViscosityRatio { turbulence_intensity, eddy_viscosity_ratio } => {
                let k = 1.5 * (turbulence_intensity * velocity_magnitude).powi(2);

                (k, c_mu * k * k / (eddy_viscosity_ratio * viscosity))
            },
            Self::AtmosphericBoundaryLayer { friction_velocity, roughness_length } => {
                let k = friction_velocity.powi(2) / c_mu.sqrt();

                (k, friction_velocity.powi(3) / (constants.kappa * (height.max(0.0) + roughness_length)))
            },
            Self::Fixed { k, epsilon } => (k, epsilon),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for the turbulence solver
pub struct TurbulenceSolverBuilder {
    pub model: TurbulenceModel,
    #[serde(default)]
    pub inlet: InletTurbulence,
    #[serde(default)]
    pub wall_treatment: WallTreatment,
    #[serde(default)]
    pub convection_scheme: ConvectionScheme,
    /// The number of Jacobi iterations used to solve the implicit transport equations in each
    /// time step.
    #[serde(default="TurbulenceSolverBuilder::default_nr_jacobi_iterations")]
    pub nr_jacobi_iterations: usize,
    /// Upper limit on the eddy viscosity, as a safeguard against unphysical values, for instance
    /// in the first time steps.
    #[serde(default="TurbulenceSolverBuilder::default_max_eddy_viscosity")]
    pub max_eddy_viscosity: Float,
}

impl TurbulenceSolverBuilder {
    pub fn default_nr_jacobi_iterations() -> usize {2}
    pub fn default_max_eddy_viscosity() -> Float {1e5}

    /// Precomputes everything the turbulence solver needs. `viscosity` is the molecular viscosity,
    /// and `no_slip_epsilon` must be the blending width used by the no-slip velocity correction.
    pub fn build_setup(
        &self,
        grid: &Grid,
        velocity_boundary_conditions: &VelocityBoundaryConditions,
        wind_environment: &WindEnvironment,
        no_slip_walls: &WallGeometries,
        slip_walls: &WallGeometries,
        viscosity: Float,
        no_slip_epsilon: Float,
    ) -> TurbulenceSolverSetup {
        let up_axis = velocity_boundary_conditions.up_axis;

        let boundary_conditions = TurbulenceBoundaryConditions::new(
            velocity_boundary_conditions,
            self.model.nr_fields(),
            |layer| {
                let mut extended_indices = [0; 3];
                extended_indices[up_axis] = layer;

                let height = wind_environment.height_from_location(
                    grid.cell_center_extended(extended_indices)
                );

                let velocity_magnitude = velocity_boundary_conditions.inlet_velocity_profile[layer].length();

                let (k, epsilon) = self.inlet.k_and_epsilon(velocity_magnitude, height, viscosity);

                self.model.fields_from_k_and_epsilon(k, epsilon)
            },
            grid
        );

        println!("Building turbulence wall treatment");
        let wall_treatment = WallTreatmentEntries::build(
            grid, no_slip_walls, slip_walls, no_slip_epsilon, self.wall_treatment
        );

        TurbulenceSolverSetup {
            model: self.model.clone(),
            boundary_conditions,
            wall_treatment,
            wall_function_constants: WallFunctionConstants::default(),
            viscosity,
            convection_scheme: self.convection_scheme,
            nr_jacobi_iterations: self.nr_jacobi_iterations,
            max_eddy_viscosity: self.max_eddy_viscosity,
        }
    }
}
