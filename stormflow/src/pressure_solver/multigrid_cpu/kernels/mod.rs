use rayon::prelude::*;
use stormath::type_aliases::Float;

use crate::grid::Grid;

pub mod jacobi;
pub mod restrict;
pub mod prolongate;
pub mod coarse_matrix;



/// Computes the Laplacian stencil applied to a value at a given extended grid index.
///
/// Returns: (1/dx²)(x[i+1] + x[i-1]) + (1/dy²)(x[j+1] + x[j-1]) + (1/dz²)(x[k+1] + x[k-1])
///          - 2(1/dx² + 1/dy² + 1/dz²) * x[i,j,k]
///
/// This is the discrete Laplacian: ∇²x ≈ Ax where A is the Poisson matrix.
///
/// Ghost-cell-based (operates on an **extended**, boundary-condition-complete array), unlike the
/// interior-only stencils above. Used only once per `solve()` call, as a final verification pass
/// on the materialized `solution` — see `MultigridCPU::solve`. Both `MultigridCPU` and
/// `MultigridGPU` call this same function on their (both extended, both materialized) final
/// solutions, so it's kept as the one shared, ghost-cell-based utility rather than also being
/// ported to the interior-only/boundary-folded form.
#[inline]
pub fn laplacian_stencil(
    grid: &Grid,
    x: &[Float],
    idx_extended: usize,
    extended_indices: [usize; 3]
) -> Float {
    let idx_xp = idx_extended + grid.extended_stride[0];
    let idx_xm = idx_extended - grid.extended_stride[0];
    let idx_yp = idx_extended + grid.extended_stride[1];
    let idx_ym = idx_extended - grid.extended_stride[1];
    let idx_zp = idx_extended + 1;
    let idx_zm = idx_extended - 1;

    // 3-point second-derivative weights per axis, for the neighbour offsets -1..=1. On a uniform
    // grid the two off-diagonal entries are both `1/h^2`.
    let weights: [&[Float; 3]; 3] = std::array::from_fn(|axis| {
        &grid.stencils[axis].second_derivative_center_low_order[extended_indices[axis]]
    });

    let off_diag = weights[0][0] * x[idx_xm] + weights[0][2] * x[idx_xp]
                 + weights[1][0] * x[idx_ym] + weights[1][2] * x[idx_yp]
                 + weights[2][0] * x[idx_zm] + weights[2][2] * x[idx_zp];

    grid.poisson_diagonal_extended(extended_indices) * x[idx_extended] + off_diag
}

/// Computes the residual r = rhs - A*x for the Poisson equation.
///
/// # Arguments
/// * `grid` - The structured grid definition
/// * `x` - Solution field on the **extended** grid (size: `nr_extended_cells`)
/// * `rhs` - Right-hand side on the **interior** grid (size: `nr_interior_cells`)
/// * `residual` - Output residual on the **interior** grid (size: `nr_interior_cells`)
///
/// # Returns
/// The sum of absolute residuals (L1 norm).
pub fn compute_residual(
    grid: &Grid,
    x: &[Float],
    rhs: &[Float],
) -> Float {
    (0..rhs.len())
        .into_par_iter()
        .map(|flat_interior| {
            // Convert interior index to extended index
            let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
            let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
            let flat_extended = grid.flat_index_on_extended_grid(extended_indices);

            // Compute A*x at this cell
            let ax = laplacian_stencil(grid, x, flat_extended, extended_indices);

            // r = rhs - A*x
            let res = rhs[flat_interior] - ax;

            res.abs()
        }).sum::<Float>() / rhs.len() as Float
}

/// 4th order accurate counterpart to `laplacian_stencil`, for `MultigridCPU`/`MultigridGPU`'s
/// final residual check. Ghost-cell-based like `laplacian_stencil` (operates on the extended,
/// boundary-condition-complete array), so — unlike `off_diagonal_sum`'s interior-only form — no
/// per-cell boundary substitution is needed: every cell, including boundary-adjacent ones, reads
/// its full `±1`/`±2` neighbors directly from the already ghost-filled array.
#[inline]
pub fn laplacian_stencil4(
    grid: &Grid,
    x: &[Float],
    idx_extended: usize,
    extended_indices: [usize; 3]
) -> Float {
    let mut result = grid.poisson_diagonal4_extended(extended_indices) * x[idx_extended];

    for axis in 0..3 {
        let stride = grid.extended_stride[axis];
        let weights = grid.poisson_axis_stencil(axis, extended_indices[axis]);

        let u_m2 = x[idx_extended - 2 * stride];
        let u_m1 = x[idx_extended - stride];
        let u_p1 = x[idx_extended + stride];
        let u_p2 = x[idx_extended + 2 * stride];

        result += weights[0] * u_m2 + weights[1] * u_m1 + weights[3] * u_p1 + weights[4] * u_p2;
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

            let ax = laplacian_stencil4(grid, x, flat_extended, extended_indices);
            let res = rhs[flat_interior] - ax;

            res.abs()
        }).sum::<Float>() / rhs.len() as Float
}
