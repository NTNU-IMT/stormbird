use stormath::type_aliases::Float;

use crate::{
    pressure_solver::boundary_conditions::PressureBoundaryConditions, 
    grid::Grid
};

const RESTRICT_WEIGHT: Float = 1.0 / 8.0;

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
    ii: usize,
    ji: usize,
    ki: usize
) -> Float {
    let off_diag = off_diagonal_sum(grid, boundary_conditions, x, idx, ii, ji, ki);
    let ax = grid.poisson_diagonal * x[idx] + off_diag;

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
/// # Safety
/// Uses unsafe pointer access to enable parallel writes. This is safe because each
/// coarse cell index is processed exactly once, so there are no data races.
pub fn compute_residual_and_restrict_kernel(
    fine_grid: &Grid,
    coarse_grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    x_fine: &[Float],
    rhs_fine: &[Float],
    flat_index_coarse_interior: usize,
) -> Float {
    // Get coarse interior indices
    let [i_c, j_c, k_c] = coarse_grid.interior_indices_from_flat_index(flat_index_coarse_interior);

    // Base fine interior indices (each coarse cell maps to 2x2x2 fine cells)
    let base_i_f = 2 * i_c;
    let base_j_f = 2 * j_c;
    let base_k_f = 2 * k_c;

    let mut restricted_value: Float = 0.0;

    // For each of the 8 fine children, compute residual and accumulate
    for &(di, dj, dk) in &RESTRICT_CHILD_OFFSETS {
        let i_f = base_i_f + di;
        let j_f = base_j_f + dj;
        let k_f = base_k_f + dk;

        let idx_fine = fine_grid.flat_index_on_interior_grid([i_f, j_f, k_f]);

        let residual_i = residual_at_interior(
            fine_grid, boundary_conditions, x_fine, rhs_fine, idx_fine, i_f, j_f, k_f
        );

        restricted_value += RESTRICT_WEIGHT * residual_i;
    }

    restricted_value   
}