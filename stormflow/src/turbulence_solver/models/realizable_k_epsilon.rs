//! The realizable k-epsilon model of Shih et al. (1995), implemented in the same way as
//! `realizableKE` in OpenFOAM.
//!
//! The transported fields are `k` and `epsilon`. The model uses two auxiliary fields, which are
//! computed once per time step from the velocity before the transport equations are solved: the
//! production of turbulent kinetic energy, `G = nu_t (grad(U) && dev(twoSymm(grad(U))))`, and the
//! coefficient of the source term in the epsilon equation, `C1 |S|`. Both are overwritten by the
//! wall functions close to the no-slip walls.

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

use crate::turbulence_solver::transport::{
    ConvectionScheme,
    velocity_gradient,
    convection_diffusion,
};
use crate::turbulence_solver::wall_treatment::{
    WallFunctionConstants,
    WallFunctionEntry
};

/// Index of the turbulent kinetic energy in the transported fields
pub const K: usize = 0;
/// Index of the dissipation rate in the transported fields
pub const EPSILON: usize = 1;

/// Index of the production of turbulent kinetic energy in the auxiliary fields
pub const PRODUCTION: usize = 0;
/// Index of `C1 |S|` in the auxiliary fields
pub const EPSILON_SOURCE: usize = 1;

/// Lower bound on k, to keep the model well defined
pub const K_MIN: Float = 1e-12;
/// Lower bound on epsilon, to keep the model well defined
pub const EPSILON_MIN: Float = 1e-12;

/// Small value added to the denominators that can become zero, as in OpenFOAM
const SMALL: Float = 1e-30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// The model coefficients, with the same default values as in OpenFOAM
pub struct RealizableKEpsilon {
    #[serde(default="RealizableKEpsilon::default_a0")]
    pub a0: Float,
    #[serde(default="RealizableKEpsilon::default_c2")]
    pub c2: Float,
    #[serde(default="RealizableKEpsilon::default_sigma_k")]
    pub sigma_k: Float,
    #[serde(default="RealizableKEpsilon::default_sigma_epsilon")]
    pub sigma_epsilon: Float,
}

impl Default for RealizableKEpsilon {
    fn default() -> Self {
        Self {
            a0: Self::default_a0(),
            c2: Self::default_c2(),
            sigma_k: Self::default_sigma_k(),
            sigma_epsilon: Self::default_sigma_epsilon(),
        }
    }
}

/// The scalar invariants of the velocity gradient that the model uses
struct GradientInvariants {
    /// `2 |dev(symm(grad(U)))|^2`
    s2: Float,
    /// `sqrt(s2)`
    mag_s: Float,
    /// `grad(U) && dev(twoSymm(grad(U)))`, which is the production divided by the eddy viscosity
    production_factor: Float,
    /// `((S & S) && S)` for `S = dev(symm(grad(U)))`
    s_cubed: Float,
    /// `|skew(grad(U))|^2`
    omega2: Float,
}

impl GradientInvariants {
    #[inline(always)]
    fn new(gradient: &[[Float; 3]; 3]) -> Self {
        let trace = gradient[0][0] + gradient[1][1] + gradient[2][2];

        let mut s = [[0.0; 3]; 3];
        let mut s2 = 0.0;
        let mut production_factor = 0.0;
        let mut omega2 = 0.0;

        for i in 0..3 {
            for j in 0..3 {
                let two_symm = gradient[i][j] + gradient[j][i];
                let dev_two_symm = if i == j { two_symm - (2.0 / 3.0) * trace } else { two_symm };

                s[i][j] = 0.5 * dev_two_symm;
                s2 += 2.0 * s[i][j] * s[i][j];

                production_factor += gradient[i][j] * dev_two_symm;

                let skew = 0.5 * (gradient[i][j] - gradient[j][i]);
                omega2 += skew * skew;
            }
        }

        let mut s_cubed = 0.0;

        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    s_cubed += s[i][j] * s[j][k] * s[k][i];
                }
            }
        }

        Self {
            s2,
            mag_s: s2.sqrt(),
            production_factor,
            s_cubed,
            omega2,
        }
    }
}

impl RealizableKEpsilon {
    pub fn default_a0() -> Float {4.0}
    pub fn default_c2() -> Float {1.9}
    pub fn default_sigma_k() -> Float {1.0}
    pub fn default_sigma_epsilon() -> Float {1.2}

    pub const FIELD_NAMES: [&'static str; 2] = ["k", "epsilon"];
    pub const NR_AUXILIARY_FIELDS: usize = 2;

    /// The coefficients and constants of the model as WGSL constants, so that the GPU version
    /// uses exactly the same values as the CPU version.
    pub fn wgsl_constants(&self) -> String {
        format!(
            "const A0: f32 = {:?};\n\
             const C2: f32 = {:?};\n\
             const INV_SIGMA_K: f32 = {:?};\n\
             const INV_SIGMA_EPSILON: f32 = {:?};\n\
             const K_MIN: f32 = {:?};\n\
             const EPSILON_MIN: f32 = {:?};\n",
            self.a0,
            self.c2,
            1.0 / self.sigma_k,
            1.0 / self.sigma_epsilon,
            K_MIN,
            EPSILON_MIN,
        )
    }

    #[inline(always)]
    /// The auxiliary fields of cell `i_0`, computed from the velocity, the eddy viscosity and the
    /// transported fields at the start of the time step. Returns `[G, C1 |S|]`.
    pub fn auxiliary_kernel(
        &self,
        i_0: usize,
        grid: &Grid,
        velocity: &[SpatialVector],
        eddy_viscosity: &[Float],
        fields_old: &[Float],
        nr_extended_cells: usize,
    ) -> [Float; 2] {
        let invariants = GradientInvariants::new(&velocity_gradient(i_0, grid, velocity));

        let k = fields_old[K * nr_extended_cells + i_0];
        let epsilon = fields_old[EPSILON * nr_extended_cells + i_0];

        let eta = invariants.mag_s * k / epsilon;
        let c1 = (eta / (5.0 + eta)).max(0.43);

        [
            eddy_viscosity[i_0] * invariants.production_factor,
            c1 * invariants.mag_s,
        ]
    }

    #[inline(always)]
    /// The wall function values for one entry, as in OpenFOAM's `epsilonWallFunction`, with the
    /// wall shear computed from the tangential velocity at the reference point of the entry.
    /// Returns `[G, epsilon]`, where the production `G` replaces the production in the cell, while
    /// epsilon is fixed to the returned value.
    pub fn wall_function_kernel(
        entry: &WallFunctionEntry,
        grid: &Grid,
        velocity: &[SpatialVector],
        fields_old: &[Float],
        nr_extended_cells: usize,
        viscosity: Float,
        constants: &WallFunctionConstants,
        y_plus_lam: Float,
    ) -> [Float; 2] {
        let k = fields_old[K * nr_extended_cells + entry.cell_index];
        let y = entry.wall_distance;

        let c_mu_25 = constants.c_mu.powf(0.25);
        let c_mu_75 = constants.c_mu.powf(0.75);

        let sqrt_k = k.sqrt();
        let y_plus = c_mu_25 * y * sqrt_k / viscosity;

        if y_plus > y_plus_lam {
            let mag_grad_u_wall = entry.tangential_velocity(velocity, grid.extended_stride) /
                entry.reference_distance;

            // The eddy viscosity at the wall from `nutkWallFunction`, plus the molecular viscosity
            let nu_wall = viscosity * y_plus * constants.kappa / (constants.e * y_plus).ln();

            [
                nu_wall * mag_grad_u_wall * c_mu_25 * sqrt_k / (constants.kappa * y),
                c_mu_75 * k * sqrt_k / (constants.kappa * y),
            ]
        } else {
            [
                0.0,
                2.0 * k * viscosity / (y * y),
            ]
        }
    }

    #[inline(always)]
    /// One Jacobi iteration of the transport equations for k and epsilon in cell `i_0`. All terms
    /// that can be treated implicitly without making the system indefinite are put on the
    /// diagonal: the time derivative, the upwind convection, the diffusion and the sink terms.
    /// The source terms are evaluated from the current iterate, `fields`. Returns `[k, epsilon]`.
    pub fn transport_kernel(
        &self,
        i_0: usize,
        grid: &Grid,
        velocity: &[SpatialVector],
        eddy_viscosity: &[Float],
        fields: &[Float],
        fields_old: &[Float],
        auxiliary: &[Float],
        nr_extended_cells: usize,
        viscosity: Float,
        time_step: Float,
        scheme: ConvectionScheme,
    ) -> [Float; 2] {
        let n = nr_extended_cells;

        let k_field = &fields[K * n..(K + 1) * n];
        let epsilon_field = &fields[EPSILON * n..(EPSILON + 1) * n];

        let k = k_field[i_0];
        let epsilon = epsilon_field[i_0];

        let inv_time_step = 1.0 / time_step;

        // --- k ---
        let k_transport = convection_diffusion(
            i_0, grid, velocity, eddy_viscosity, k_field, viscosity, 1.0 / self.sigma_k, scheme
        );

        let k_diagonal = inv_time_step + k_transport.diagonal + epsilon / k;
        let k_rhs = fields_old[K * n + i_0] * inv_time_step +
            auxiliary[PRODUCTION * n + i_0] +
            k_transport.neighbors;

        // --- epsilon ---
        let epsilon_transport = convection_diffusion(
            i_0, grid, velocity, eddy_viscosity, epsilon_field, viscosity, 1.0 / self.sigma_epsilon, scheme
        );

        let epsilon_diagonal = inv_time_step + epsilon_transport.diagonal +
            self.c2 * epsilon / (k + (viscosity * epsilon).sqrt());
        let epsilon_rhs = fields_old[EPSILON * n + i_0] * inv_time_step +
            auxiliary[EPSILON_SOURCE * n + i_0] * epsilon +
            epsilon_transport.neighbors;

        [
            (k_rhs / k_diagonal).max(K_MIN),
            (epsilon_rhs / epsilon_diagonal).max(EPSILON_MIN),
        ]
    }

    #[inline(always)]
    /// The eddy viscosity in cell `i_0`, `nu_t = C_mu k^2 / epsilon`, where `C_mu` is computed from
    /// the velocity gradient, k and epsilon, as in OpenFOAM's `realizableKE::rCmu`. Limited to
    /// `max_eddy_viscosity`.
    pub fn eddy_viscosity_kernel(
        &self,
        i_0: usize,
        grid: &Grid,
        velocity: &[SpatialVector],
        fields: &[Float],
        nr_extended_cells: usize,
        max_eddy_viscosity: Float,
    ) -> Float {
        let invariants = GradientInvariants::new(&velocity_gradient(i_0, grid, velocity));

        let k = fields[K * nr_extended_cells + i_0];
        let epsilon = fields[EPSILON * nr_extended_cells + i_0];

        let w = (2.0 * Float::sqrt(2.0)) * invariants.s_cubed /
            (invariants.mag_s * invariants.s2 + SMALL);

        let phi_s = (1.0 / 3.0) * (Float::sqrt(6.0) * w).clamp(-1.0, 1.0).acos();
        let a_s = Float::sqrt(6.0) * phi_s.cos();
        let u_s = (0.5 * invariants.s2 + invariants.omega2).sqrt();

        let c_mu = 1.0 / (self.a0 + a_s * u_s * k / epsilon);

        (c_mu * k * k / epsilon).min(max_eddy_viscosity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without any velocity gradients, C_mu is 1 / A0, as in OpenFOAM
    #[test]
    fn eddy_viscosity_in_uniform_flow() {
        let grid = Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [4, 4, 4]);
        let n = grid.nr_extended_cells();

        let model = RealizableKEpsilon::default();

        let velocity = vec![SpatialVector([3.0, 1.0, 0.0]); n];

        let (k, epsilon) = (0.5, 0.02);

        let mut fields = vec![k; 2 * n];
        fields[n..].fill(epsilon);

        let i_0 = grid.flat_index_on_extended_grid_from_interior_indices([1, 2, 1]);

        let nu_t = model.eddy_viscosity_kernel(i_0, &grid, &velocity, &fields, n, 1e10);

        let expected = k * k / (model.a0 * epsilon);

        assert!((nu_t - expected).abs() < 1e-5 * expected, "{nu_t} vs {expected}");
    }

    /// In a pure shear flow, the realizable C_mu is lower than the uniform flow value
    #[test]
    fn eddy_viscosity_is_reduced_by_shear() {
        let grid = Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [4, 4, 4]);
        let n = grid.nr_extended_cells();

        let model = RealizableKEpsilon::default();

        let velocity: Vec<SpatialVector> = (0..n).map(|i| {
            let center = grid.cell_center_extended(grid.extended_indices_from_flat_index(i));

            SpatialVector([2.0 * center[1], 0.0, 0.0])
        }).collect();

        let (k, epsilon) = (0.5, 0.02);

        let mut fields = vec![k; 2 * n];
        fields[n..].fill(epsilon);

        let i_0 = grid.flat_index_on_extended_grid_from_interior_indices([1, 2, 1]);

        let nu_t = model.eddy_viscosity_kernel(i_0, &grid, &velocity, &fields, n, 1e10);

        let uniform_value = k * k / (model.a0 * epsilon);

        assert!(nu_t > 0.0 && nu_t < uniform_value, "{nu_t} vs {uniform_value}");
    }
}
