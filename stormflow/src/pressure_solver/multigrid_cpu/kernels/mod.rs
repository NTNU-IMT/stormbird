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
pub fn compute_residual4(
    grid: &Grid,
    x: &[Float],
    rhs: &[Float],
) -> Float {
    (0..rhs.len())
        .into_par_iter()
        .map(|flat_interior| {
            let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
            let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
            let flat_extended = grid.flat_index_on_extended_grid(extended_indices);

            let ax = laplacian_stencil4(grid, x, flat_extended);
            let res = rhs[flat_interior] - ax;

            res.abs()
        }).sum::<Float>() / rhs.len() as Float
}
