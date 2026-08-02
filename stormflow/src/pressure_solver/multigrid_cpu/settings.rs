use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// How the coarsest multigrid level's Poisson equation is solved at the bottom of each V-cycle.
pub enum CoarsestLevelSolver {
    /// Solve exactly via Gaussian elimination, using a dense matrix assembled once from the same
    /// boundary-folded stencil the Jacobi smoother iterates towards (see
    /// `multigrid_cpu::kernels::coarse_matrix::build_poisson_matrix4`). `MultigridGPU` reuses this
    /// same CPU-side solve: it reads the restricted RHS back from the GPU, solves on the CPU, and
    /// writes the solution back before prolongation.
    #[default]
    Exact,
    /// Solve approximately via `nr_smooth_iterations * 4` extra Jacobi iterations (matching each
    /// level's smoother, just run longer), staying entirely on whichever device (CPU/GPU) is
    /// already running the rest of the V-cycle.
    Jacobi,
}

#[derive(Debug, Clone)]
pub struct MultigridSettings {
    pub nr_v_cycles: usize,
    pub nr_smooth_iterations: usize,
    pub compute_residual_after_solve: bool,
    pub coarsest_level_solver: CoarsestLevelSolver,
}
