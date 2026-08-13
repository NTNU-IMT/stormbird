use rayon::prelude::*;
use stormath::type_aliases::Float;

use crate::grid::Grid;

pub mod jacobi;
pub mod restrict;
pub mod prolongate;
pub mod coarse_matrix;

/// 4th order accurate counterpart to `laplacian_stencil`, for `MultigridCPU`/`MultigridGPU`'s
/// final residual check. Ghost-cell-based like `laplacian_stencil` (operates on the extended,
/// boundary-condition-complete array), so — unlike `off_diagonal_sum`'s interior-only form — no
/// per-cell boundary substitution is needed: every cell, including boundary-adjacent ones, reads
/// its full `±1`/`±2` neighbors directly from the already ghost-filled array.
#[inline]
pub fn laplacian_stencil4(
    grid: &Grid,
    x: &[Float],
    idx_extended: usize
) -> Float {
    let mut result = grid.poisson_diagonal4 * x[idx_extended];

    for axis in 0..3 {
        let stride = grid.extended_stride[axis];
        let inv_dx2 = grid.inv_cell_length_squared[axis];

        let u_m2 = x[idx_extended - 2 * stride];
        let u_m1 = x[idx_extended - stride];
        let u_p1 = x[idx_extended + stride];
        let u_p2 = x[idx_extended + 2 * stride];

        result += inv_dx2 * ((4.0 / 3.0) * (u_m1 + u_p1) - (1.0 / 12.0) * (u_m2 + u_p2));
    }

    result
}

/// 4th order accurate counterpart to `compute_residual`, used by `MultigridCPU`/`MultigridGPU`
/// (both now solving with the 4th order stencil) instead of the 2nd order `compute_residual`,
/// which remains as-is for `FftCPU`.
///
/// `exclude` marks interior cells (by flat interior index) to leave out of the average — meant for
/// cells whose pressure value is blended with a mirrored image point rather than purely solved for
/// (e.g. `jacobi::jacobi_kernel_with_slip_correction`'s slip-wall zero-gradient correction). Such
/// cells don't satisfy `Ax = rhs` by construction, so including them would report a permanently
/// nonzero "residual" that reflects the deliberate boundary correction rather than how well the
/// solve actually converged elsewhere. Pass `&[]` (as
/// `MultigridGPU` does, since it has no such correction) to include every cell, matching the
/// previous unconditional behavior.
pub fn compute_residual4(
    grid: &Grid,
    x: &[Float],
    rhs: &[Float],
    exclude: &[bool],
) -> Float {
    let nr_excluded = exclude.iter().filter(|&&excluded| excluded).count();
    let nr_included = rhs.len() - nr_excluded;

    if nr_included == 0 {
        return 0.0;
    }

    (0..rhs.len())
        .into_par_iter()
        .filter(|&flat_interior| !exclude.get(flat_interior).copied().unwrap_or(false))
        .map(|flat_interior| {
            let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
            let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
            let flat_extended = grid.flat_index_on_extended_grid(extended_indices);

            let ax = laplacian_stencil4(grid, x, flat_extended);
            let res = rhs[flat_interior] - ax;

            res.abs()
        }).sum::<Float>() / nr_included as Float
}
