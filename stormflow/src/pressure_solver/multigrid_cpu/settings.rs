use serde::{Serialize, Deserialize};

use super::zero_gradient_stencils::ZeroGradientInterpolationOrder;
use crate::geometry::Geometry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// How the coarsest multigrid level's Poisson equation is solved at the bottom of each V-cycle.
pub enum CoarsestLevelSolver {
    /// Solve exactly via Gaussian elimination, using a dense matrix assembled once from the same
    /// boundary-folded stencil the Jacobi smoother iterates towards (see
    /// `multigrid_cpu::kernels::coarse_matrix::build_poisson_matrix4`). `MultigridGPU` reuses this
    /// same CPU-side solve: it reads the restricted RHS back from the GPU, solves on the CPU, and
    /// writes the solution back before prolongation.
    ///
    /// The cost of the solve, and the memory needed to store the dense matrix, grows quickly
    /// with the number of cells on the coarsest level, so this can be very slow, and use a lot of
    /// memory, if the grid cannot be coarsened much.
    Exact,
    /// Solve approximately via `nr_smooth_iterations * 4` extra Jacobi iterations (matching each
    /// level's smoother, just run longer), staying entirely on whichever device (CPU/GPU) is
    /// already running the rest of the V-cycle. Default.
    #[default]
    Jacobi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// Which walls get a zero-gradient (Neumann) boundary condition on the pressure, enforced by
/// mirroring the pressure across the wall surface for the cells just inside each geometry (see
/// `zero_gradient_stencils`). This is independent of how the walls are represented in the velocity
/// solver: slip walls use a mirror construction on the velocity, while no-slip walls use data
/// immersion. Without this condition, the walls only affect the pressure through the velocity
/// field.
pub enum ZeroGradientOnWalls {
    /// No walls get the zero-gradient condition. Default.
    #[default]
    NotUsed,
    /// Only the slip geometries get the zero-gradient condition
    SlipWallsOnly,
    /// Only the no-slip geometries get the zero-gradient condition
    NoSlipWallsOnly,
    /// Both the slip and the no-slip geometries get the zero-gradient condition
    AllWalls,
}

impl ZeroGradientOnWalls {
    pub fn is_used(&self) -> bool {
        *self != Self::NotUsed
    }

    /// Returns the geometries that should get the zero-gradient condition
    pub fn wall_geometries(
        &self,
        slip_geometries: &[Geometry],
        no_slip_geometries: &[Geometry]
    ) -> Vec<Geometry> {
        match self {
            Self::NotUsed => Vec::new(),
            Self::SlipWallsOnly => slip_geometries.to_vec(),
            Self::NoSlipWallsOnly => no_slip_geometries.to_vec(),
            Self::AllWalls => slip_geometries.iter().chain(no_slip_geometries).cloned().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultigridSettings {
    pub nr_v_cycles: usize,
    pub nr_smooth_iterations: usize,
    pub compute_residual_after_solve: bool,
    pub coarsest_level_solver: CoarsestLevelSolver,
    /// Which walls get a zero-gradient boundary condition on the pressure. Experimental, and not
    /// used by default.
    pub zero_gradient_on_walls: ZeroGradientOnWalls,
    /// Interpolation order used to sample the pressure at the mirrored image point for the
    /// zero-gradient condition on walls specifically (ignored when `zero_gradient_on_walls` is
    /// `NotUsed`). Does not affect the rest of the pressure solve, which always uses 4th order
    /// stencils.
    pub zero_gradient_interpolation_order: ZeroGradientInterpolationOrder,
}
