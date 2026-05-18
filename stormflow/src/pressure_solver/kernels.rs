use crate::grid::Grid;

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
    inv_dx2: Float,
    inv_dy2: Float,
    inv_dz2: Float,
    inv_diag: Float,
    one_minus_omega: Float,
    omega: Float,
    stride_x: usize,
    stride_y: usize,
) {
    new.par_iter_mut()
        .enumerate()
        .for_each(|(idx_extended, new_val)| {
            // Check if this extended index corresponds to an interior cell
            // by converting back to 3D indices and checking bounds
            let extended_indices = grid.extended_indices_from_flat_index(idx_extended);
            let [ei, ej, ek] = extended_indices;
            
            // Interior cells are offset by 1 from extended grid boundaries
            // Check if this is an interior cell (not a ghost cell)
            let is_interior = ei >= 1 && ei < grid.extended_shape[0] - 1
                           && ej >= 1 && ej < grid.extended_shape[1] - 1
                           && ek >= 1 && ek < grid.extended_shape[2] - 1;
            
            if !is_interior {
                return; // Skip ghost cells - leave them unchanged
            }
            
            // Convert extended indices to interior indices
            let interior_indices = [ei - 1, ej - 1, ek - 1];
            let flat_interior = grid.flat_index_on_interior_grid(interior_indices);
            
            // Compute flat indices for 6 neighbors on extended grid
            let idx_current = idx_extended;
            let idx_xp = idx_current + stride_x;  // i+1
            let idx_xm = idx_current - stride_x;  // i-1
            let idx_yp = idx_current + stride_y;  // j+1
            let idx_ym = idx_current - stride_y;  // j-1
            let idx_zp = idx_current + 1;         // k+1
            let idx_zm = idx_current - 1;         // k-1
            
            // Compute off-diagonal contribution (sum of neighbor contributions)
            let off_diag = inv_dx2 * (current[idx_xp] + current[idx_xm])
                         + inv_dy2 * (current[idx_yp] + current[idx_ym])
                         + inv_dz2 * (current[idx_zp] + current[idx_zm]);
            
            // Standard Jacobi update: p_new = (rhs - off_diag) / diag
            let jacobi_update = (rhs[flat_interior] - off_diag) * inv_diag;
            
            // Weighted Jacobi: p_new = (1 - ω) * p_old + ω * jacobi_update
            *new_val = one_minus_omega * current[idx_current] + omega * jacobi_update;
        });
}

/// Performs weighted Jacobi smoothing iterations for the Poisson equation on a structured grid.
/// 
/// This function directly applies the 7-point Laplacian stencil without building a sparse matrix,
/// which is more efficient for structured grids. The discretization is:
/// 
/// ```text
/// ∇²p ≈ (p[i+1,j,k] + p[i-1,j,k])/dx² + (p[i,j+1,k] + p[i,j-1,k])/dy² 
///      + (p[i,j,k+1] + p[i,j,k-1])/dz² - 2(1/dx² + 1/dy² + 1/dz²)p[i,j,k] = rhs
/// ```
/// 
/// The weighted Jacobi update is:
/// ```text
/// p_new = (1 - ω) * p_old + ω * D⁻¹(rhs - off_diagonal_terms)
/// ```
/// 
/// # Arguments
/// * `grid` - The structured grid definition
/// * `rhs` - Right-hand side vector on the **interior** grid (size: `nr_interior_cells`)
/// * `solution` - Pressure field on the **extended** grid (size: `nr_extended_cells`). 
///                Ghost cells should have boundary values set before calling this function.
/// * `work` - Scratch buffer on the **extended** grid for intermediate computations
/// * `nr_iterations` - Number of Jacobi iterations to perform
/// * `omega` - Relaxation weight:
///   - ω = 1.0: Standard Jacobi
///   - ω = 2/3 ≈ 0.667: Optimal for Laplacian on structured grids
pub fn poisson_jacobi_smoother(
    grid: &Grid,
    rhs: &[Float], 
    solution: &mut [Float], 
    work: &mut [Float], 
    nr_iterations: usize,
    omega: Float
) {
    work.copy_from_slice(solution);
    
    let [dx, dy, dz] = grid.cell_length.0;
    
    // Precompute stencil coefficients
    let inv_dx2 = 1.0 / (dx * dx);
    let inv_dy2 = 1.0 / (dy * dy);
    let inv_dz2 = 1.0 / (dz * dz);
    
    // Diagonal coefficient: -2/dx² - 2/dy² - 2/dz²
    let inv_diag = 1.0 / (-2.0 * (inv_dx2 + inv_dy2 + inv_dz2));
    
    let one_minus_omega = 1.0 - omega;
    
    // Precompute strides for extended grid indexing
    let stride_x = grid.extended_shape[1] * grid.extended_shape[2];
    let stride_y = grid.extended_shape[2];
    
    for iteration in 0..nr_iterations {
        // Swap buffers: read from current, write to new
        // Even iterations: read from solution, write to work
        // Odd iterations: read from work, write to solution
        if iteration % 2 == 0 {
            jacobi_iteration_step(
                grid, rhs, solution, work,
                inv_dx2, inv_dy2, inv_dz2, inv_diag,
                one_minus_omega, omega,
                stride_x, stride_y,
            );
        } else {
            jacobi_iteration_step(
                grid, rhs, work, solution,
                inv_dx2, inv_dy2, inv_dz2, inv_diag,
                one_minus_omega, omega,
                stride_x, stride_y,
            );
        }
    }
    
    // If odd number of iterations, the result is in work; copy back to solution
    if nr_iterations % 2 == 1 {
        // Only copy interior cells (ghost cells don't need updating)
        for flat_interior in 0..grid.nr_interior_cells() {
            let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
            let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
            let idx_ext = extended_indices[0] * stride_x 
                        + extended_indices[1] * stride_y 
                        + extended_indices[2];
            solution[idx_ext] = work[idx_ext];
        }
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