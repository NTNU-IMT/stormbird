use stormath::type_aliases::Float;

use crate::{
    pressure_solver::boundary_conditions::PressureBoundaryConditions, 
    grid::Grid
};

const RESTRICT_WEIGHT: Float = 0.125;

const RESTRICT_CHILD_OFFSETS: [(usize, usize, usize); 8] = [
    (0, 0, 0),
    (0, 0, 1),
    (0, 1, 0),
    (0, 1, 1),
    (1, 0, 0),
    (1, 0, 1),
    (1, 1, 0),
    (1, 1, 1),
];

use super::jacobi::off_diagonal_sum;

/// Residual r = rhs - A*x at interior cell (ii,ji,ki)/idx, with a missing neighbor on any axis
/// explicitly substituted the same way `off_diagonal_sum` does for the smoother. Used by
/// `MultigridCPU::compute_residual_and_restrict`, which operates mid-solve on the interior-only
/// `x_at_levels` (unlike `compute_residual` below, which runs once at the very end on the
/// materialized, ghost-cell-complete `solution`).
#[inline]
pub fn residual_at_interior(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    x: &[Float],
    rhs: &[Float],
    idx: usize,
    indices: [usize; 3]
) -> Float {
    let off_diag = off_diagonal_sum(grid, boundary_conditions, x, idx, indices);
    let ax = grid.poisson_diagonal4 * x[idx] + off_diag;

    rhs[idx] - ax
}

#[inline(always)]
/// Computes residuals at the fine level and restricts them to the coarse level in a single fused pass.
///
/// This avoids storing the full residual array at the fine level, reducing memory traffic.
/// For each coarse cell, computes residuals for its 8 fine children on-the-fly and averages them.
///
/// The residual for fine cell i is: r_i = rhs_i - (A * x)_i
/// where A is the discrete Laplacian (Poisson matrix), with boundary conditions folded
/// directly into the stencil for fine cells missing a real neighbor.
///
/// # Grid layout
/// - `x_fine`/`rhs_fine` are on the **interior** grid
/// - `rhs_coarse` (output) is on the **interior** grid
///
pub fn compute_residual_and_restrict_kernel(
    indices_coarse: [usize; 3],
    grid_fine: &Grid,
    x_fine: &[Float],
    rhs_fine: &[Float],
    boundary_conditions: &PressureBoundaryConditions,
) -> Float {
    let [i_c, j_c, k_c] = indices_coarse;

    // Base fine interior indices (each coarse cell maps to 2x2x2 fine cells)
    let base_i_f = 2 * i_c;
    let base_j_f = 2 * j_c;
    let base_k_f = 2 * k_c;

    RESTRICT_CHILD_OFFSETS.iter().map(
        |(di, dj, dk)| {
            let i_f = base_i_f + di;
            let j_f = base_j_f + dj;
            let k_f = base_k_f + dk;
    
            let idx_fine = grid_fine.flat_index_on_interior_grid([i_f, j_f, k_f]);
    
            let residual = residual_at_interior(
                grid_fine, boundary_conditions, x_fine, rhs_fine, idx_fine, [i_f, j_f, k_f]
            );
    
            RESTRICT_WEIGHT * residual
        }
    ).sum::<Float>()
}