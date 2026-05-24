use crate::{
    boundary_conditions::pressure::PressureBoundaryConditions, 
    grid::Grid
};

use rayon::prelude::*;
use stormath::type_aliases::Float;

/// Performs a single Jacobi iteration step for the Poisson equation.
/// 
/// Reads from `current` buffer and writes to `new` buffer.
fn jacobi_iteration_step(
    grid: &Grid,
    rhs: &[Float],
    current: &[Float],
    new: &mut [Float],
    omega: Float,
) {
    new.par_chunks_mut(grid.extended_stride[0])
        .enumerate()
        .for_each(|(i, plane)| {
            // i is the extended-grid i index. Skip halo planes i == 0 and i == nxi + 1.
            if i == 0 || i > grid.interior_shape[0] {
                return;
            }
            let ii = i - 1; // interior i index

            for ji in 0..grid.interior_shape[1] {
                let j = ji + 1;

                // Index of (i, j, k=1) within THIS plane: subtract the plane's base (i * stride_x).
                let base_extended = grid.flat_index_on_extended_grid([i, j, 1]);
                let plane_base = base_extended - i * grid.extended_stride[0];
                let base_interior = grid.flat_index_on_interior_grid([ii, ji, 0]);

                // The contiguous k-column we WRITE lives entirely inside `plane`.
                let out_col = &mut plane[plane_base..plane_base + grid.interior_shape[2]];

                for k in 0..grid.interior_shape[2] {
                    let i_0 = base_extended + k;
                    let off_diag =
                          grid.inv_cell_length_squared[0] * (current[i_0 + grid.extended_stride[0]] + current[i_0 - grid.extended_stride[0]])
                        + grid.inv_cell_length_squared[1] * (current[i_0 + grid.extended_stride[1]] + current[i_0 - grid.extended_stride[1]])
                        + grid.inv_cell_length_squared[2] * (current[i_0 + 1]        + current[i_0 - 1]);

                    let jacobi_update = (rhs[base_interior + k] - off_diag) * grid.poisson_diagonal;
                    out_col[k] = (1.0 - omega) * current[i_0] + omega * jacobi_update;
                }
            }
        });
}

pub fn poisson_jacobi_smoother(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    rhs: &[Float], 
    solution: &mut [Float], 
    work: &mut [Float], 
    nr_iterations: usize,
    omega: Float
) {
    work.copy_from_slice(solution);

    boundary_conditions.set_ghost_cells(grid, work);
    boundary_conditions.set_ghost_cells(grid, solution);
    
    for iteration in 0..nr_iterations {
        // Swap buffers: read from current, write to new
        // Even iterations: read from solution, write to work
        // Odd iterations: read from work, write to solution
        if iteration % 2 == 0 {
            jacobi_iteration_step(grid, rhs, solution, work, omega);

            boundary_conditions.set_ghost_cells(grid, work);
        } else {
            jacobi_iteration_step(grid, rhs, work, solution, omega);

            boundary_conditions.set_ghost_cells(grid, solution);
        }
    }
    
    // If odd number of iterations, the result is in work; copy back to solution
    if nr_iterations % 2 == 1 {
        solution.copy_from_slice(&work);
    }
}

/// Computes the Laplacian stencil applied to a value at a given extended grid index.
/// 
/// Returns: (1/dx²)(x[i+1] + x[i-1]) + (1/dy²)(x[j+1] + x[j-1]) + (1/dz²)(x[k+1] + x[k-1]) 
///          - 2(1/dx² + 1/dy² + 1/dz²) * x[i,j,k]
/// 
/// This is the discrete Laplacian: ∇²x ≈ Ax where A is the Poisson matrix.
#[inline]
pub fn apply_laplacian_stencil(
    x: &[Float],
    idx_extended: usize,
    inv_dx2: Float,
    inv_dy2: Float,
    inv_dz2: Float,
    diag: Float,
    stride_x: usize,
    stride_y: usize,
) -> Float {
    let idx_xp = idx_extended + stride_x;
    let idx_xm = idx_extended - stride_x;
    let idx_yp = idx_extended + stride_y;
    let idx_ym = idx_extended - stride_y;
    let idx_zp = idx_extended + 1;
    let idx_zm = idx_extended - 1;
    
    let off_diag = inv_dx2 * (x[idx_xp] + x[idx_xm])
                 + inv_dy2 * (x[idx_yp] + x[idx_ym])
                 + inv_dz2 * (x[idx_zp] + x[idx_zm]);
    
    diag * x[idx_extended] + off_diag
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
    let [dx, dy, dz] = grid.cell_length.0;
    
    // Precompute stencil coefficients
    let inv_dx2 = 1.0 / (dx * dx);
    let inv_dy2 = 1.0 / (dy * dy);
    let inv_dz2 = 1.0 / (dz * dz);
    let diag = -2.0 * (inv_dx2 + inv_dy2 + inv_dz2);
    
    (0..rhs.len())
        .into_par_iter()
        .map(|flat_interior| {
            // Convert interior index to extended index
            let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
            let [i, j, k] = interior_indices;
            let idx_extended = (i + 1) * grid.extended_stride[0] + (j + 1) * grid.extended_stride[1] + (k + 1);
            
            // Compute A*x at this cell
            let ax = apply_laplacian_stencil(
                x, idx_extended,
                inv_dx2, inv_dy2, inv_dz2, diag,
                grid.extended_stride[0], grid.extended_stride[1]
            );
            
            // r = rhs - A*x
            let res = rhs[flat_interior] - ax;
            res.abs()
        }).sum::<Float>() / rhs.len() as Float
}
