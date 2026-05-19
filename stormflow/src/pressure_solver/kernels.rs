use crate::{
    boundary_conditions::BoundaryConditions, 
    grid::Grid
};

use rayon::prelude::*;
use stormath::type_aliases::Float;

/// Performs a single Jacobi iteration step for the Poisson equation.
/// 
/// Reads from `current` buffer and writes to `new` buffer.
#[inline]
fn jacobi_iteration_step(
    grid: &Grid,
    rhs: &[Float],
    current: &[Float],
    new: &mut [Float],
    omega: Float,
) {
    let [dx, dy, dz] = grid.cell_length.0;
    
    // Precompute stencil coefficients
    let inv_dx2 = 1.0 / (dx * dx);
    let inv_dy2 = 1.0 / (dy * dy);
    let inv_dz2 = 1.0 / (dz * dz);
    
    // Diagonal coefficient: -2/dx² - 2/dy² - 2/dz²
    let inv_diag = 1.0 / (-2.0 * (inv_dx2 + inv_dy2 + inv_dz2));
    
    let one_minus_omega = 1.0 - omega;
    
    new.par_iter_mut()
        .enumerate()
        .for_each(|(i0, new_val)| {
            let [i, j, k] = grid.extended_indices_from_flat_index(i0);
            let is_interior = grid.is_cell_interior([i, j, k]);
            
            if !is_interior {
                return; // Skip ghost cells - leave them unchanged
            }

            let interior_indices = grid.interior_indices_from_extended_indices([i, j, k]);
            let flat_interior = grid.flat_index_on_interior_grid(interior_indices);

            let i_xp = grid.flat_index_on_extended_grid([i+1, j, k]);
            let i_xm = grid.flat_index_on_extended_grid([i-1, j, k]);
            let i_yp = grid.flat_index_on_extended_grid([i, j+1, k]);
            let i_ym = grid.flat_index_on_extended_grid([i, j-1, k]);
            let i_zp = grid.flat_index_on_extended_grid([i, j, k+1]);
            let i_zm = grid.flat_index_on_extended_grid([i, j, k-1]);
            
            // Compute off-diagonal contribution (sum of neighbor contributions)
            let off_diag = inv_dx2 * (current[i_xp] + current[i_xm])
                         + inv_dy2 * (current[i_yp] + current[i_ym])
                         + inv_dz2 * (current[i_zp] + current[i_zm]);
            
            // Standard Jacobi update: p_new = (rhs - off_diag) / diag
            let jacobi_update = (rhs[flat_interior] - off_diag) * inv_diag;
            
            // Weighted Jacobi: p_new = (1 - ω) * p_old + ω * jacobi_update
            *new_val = one_minus_omega * current[i0] + omega * jacobi_update;
        });
}

pub fn poisson_jacobi_smoother(
    grid: &Grid,
    boundary_conditions: &BoundaryConditions,
    rhs: &[Float], 
    solution: &mut [Float], 
    work: &mut [Float], 
    nr_iterations: usize,
    omega: Float
) {
    work.copy_from_slice(solution);

    boundary_conditions.set_pressure_ghost_cells(grid, work);
    boundary_conditions.set_pressure_ghost_cells(grid, solution);
    
    for iteration in 0..nr_iterations {
        // Swap buffers: read from current, write to new
        // Even iterations: read from solution, write to work
        // Odd iterations: read from work, write to solution
        if iteration % 2 == 0 {
            jacobi_iteration_step(grid, rhs, solution, work, omega);

            boundary_conditions.set_pressure_ghost_cells(grid, work);
        } else {
            jacobi_iteration_step(grid, rhs, work, solution, omega);

            boundary_conditions.set_pressure_ghost_cells(grid, solution);
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
    residual: &mut [Float],
) -> Float {
    let [dx, dy, dz] = grid.cell_length.0;
    
    // Precompute stencil coefficients
    let inv_dx2 = 1.0 / (dx * dx);
    let inv_dy2 = 1.0 / (dy * dy);
    let inv_dz2 = 1.0 / (dz * dz);
    let diag = -2.0 * (inv_dx2 + inv_dy2 + inv_dz2);
    
    // Strides for extended grid
    let stride_x = grid.extended_shape[1] * grid.extended_shape[2];
    let stride_y = grid.extended_shape[2];
    
    let residual_sum: Float = residual
        .par_iter_mut()
        .enumerate()
        .map(|(flat_interior, res)| {
            // Convert interior index to extended index
            let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
            let [i, j, k] = interior_indices;
            let idx_extended = (i + 1) * stride_x + (j + 1) * stride_y + (k + 1);
            
            // Compute A*x at this cell
            let ax = apply_laplacian_stencil(
                x, idx_extended,
                inv_dx2, inv_dy2, inv_dz2, diag,
                stride_x, stride_y
            );
            
            // r = rhs - A*x
            *res = rhs[flat_interior] - ax;
            res.abs()
        })
        .sum();
    
    residual_sum
}
