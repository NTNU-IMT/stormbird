use rayon::prelude::*;
use stormath::type_aliases::Float;

use crate::grid::Grid;

pub mod jacobi;
pub mod restrict;
pub mod prolongate;



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
    idx_extended: usize
) -> Float {
    let idx_xp = idx_extended + grid.extended_stride[0];
    let idx_xm = idx_extended - grid.extended_stride[0];
    let idx_yp = idx_extended + grid.extended_stride[1];
    let idx_ym = idx_extended - grid.extended_stride[1];
    let idx_zp = idx_extended + 1;
    let idx_zm = idx_extended - 1;

    let off_diag = grid.inv_cell_length_squared[0] * (x[idx_xp] + x[idx_xm])
                 + grid.inv_cell_length_squared[1] * (x[idx_yp] + x[idx_ym])
                 + grid.inv_cell_length_squared[2] * (x[idx_zp] + x[idx_zm]);

    grid.poisson_diagonal * x[idx_extended] + off_diag
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
            let ax = laplacian_stencil(grid, x, flat_extended);

            // r = rhs - A*x
            let res = rhs[flat_interior] - ax;

            res.abs()
        }).sum::<Float>() / rhs.len() as Float
}
