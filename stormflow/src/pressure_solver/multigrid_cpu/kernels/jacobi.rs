use crate::{
    grid::Grid,
    pressure_solver::boundary_conditions::PressureBoundaryConditions
};

use rayon::prelude::*;
use stormath::type_aliases::Float;

pub const JACOBI_WEIGHT: Float = 0.66666667;

#[inline]
fn boundary_sign(boundary_conditions: &PressureBoundaryConditions, axis: usize, face: usize) -> Float {
    if boundary_conditions.condition(axis, face).as_gpu_flag() == 1 { -1.0 } else { 1.0 }
}

/// Off-diagonal stencil sum for the interior cell at (ii,ji,ki)/idx. For every axis where a real
/// neighbor exists it's read directly; where it doesn't (a boundary-adjacent cell), the missing
/// read is replaced by `current[idx]` itself, signed per the corresponding face's boundary
/// condition — the "boundary-folded stencil" technique, mathematically equivalent to a ghost cell
/// but without maintaining a padded array. See `multigrid_gpu`'s `jacobi_shader.wgsl` for the
/// same technique on the GPU side; the two must stay in sync.
///
/// This is an *explicit* substitution using the already-known `current[idx]` (matching the ghost
/// cell a separate pass would have produced from the *previous* Jacobi sweep), not an implicit
/// one — it belongs in the off-diagonal sum, not folded into the diagonal coefficient, since it
/// isn't solving for the new value self-referentially.
#[inline]
pub fn off_diagonal_sum(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    current: &[Float],
    idx: usize,
    ii: usize,
    ji: usize,
    ki: usize
) -> Float {
    let [nx, ny, nz] = grid.interior_shape;
    let [sx, sy, _sz] = grid.interior_stride;

    let mut off_diag: Float = 0.0;

    if ii > 0 {
        off_diag += grid.inv_cell_length_squared[0] * current[idx - sx];
    } else {
        off_diag += grid.inv_cell_length_squared[0] * boundary_sign(boundary_conditions, 0, 0) * current[idx];
    }
    if ii + 1 < nx {
        off_diag += grid.inv_cell_length_squared[0] * current[idx + sx];
    } else {
        off_diag += grid.inv_cell_length_squared[0] * boundary_sign(boundary_conditions, 0, 1) * current[idx];
    }

    if ji > 0 {
        off_diag += grid.inv_cell_length_squared[1] * current[idx - sy];
    } else {
        off_diag += grid.inv_cell_length_squared[1] * boundary_sign(boundary_conditions, 1, 0) * current[idx];
    }
    if ji + 1 < ny {
        off_diag += grid.inv_cell_length_squared[1] * current[idx + sy];
    } else {
        off_diag += grid.inv_cell_length_squared[1] * boundary_sign(boundary_conditions, 1, 1) * current[idx];
    }

    if ki > 0 {
        off_diag += grid.inv_cell_length_squared[2] * current[idx - 1];
    } else {
        off_diag += grid.inv_cell_length_squared[2] * boundary_sign(boundary_conditions, 2, 0) * current[idx];
    }
    if ki + 1 < nz {
        off_diag += grid.inv_cell_length_squared[2] * current[idx + 1];
    } else {
        off_diag += grid.inv_cell_length_squared[2] * boundary_sign(boundary_conditions, 2, 1) * current[idx];
    }

    off_diag
}

/// Performs a single Jacobi iteration step for the Poisson equation. `rhs`/`current`/`new` are
/// all sized to the **interior** grid — there are no ghost cells; boundary conditions are folded
/// directly into the stencil via `off_diagonal_sum`.
///
/// Reads from `current` buffer and writes to `new` buffer.
pub fn jacobi_iteration_step(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    rhs: &[Float],
    current: &[Float],
    new: &mut [Float]
) {
    let [_nx, ny, nz] = grid.interior_shape;
    let [_sx, sy, _sz] = grid.interior_stride;

    new.par_chunks_mut(grid.interior_stride[0])
        .enumerate()
        .for_each(|(ii, plane)| {
            for ji in 0..ny {
                let row_base = ji * sy;
                let out_row = &mut plane[row_base..row_base + nz];

                for ki in 0..nz {
                    let idx = ii * grid.interior_stride[0] + row_base + ki;

                    let off_diag = off_diagonal_sum(
                        grid, 
                        boundary_conditions, 
                        current, 
                        idx, 
                        ii, 
                        ji, 
                        ki
                    );
                    
                    let jacobi_update = (rhs[idx] - off_diag) * grid.poisson_inv_diagonal;

                    out_row[ki] = (1.0 - JACOBI_WEIGHT) * current[idx] + JACOBI_WEIGHT * jacobi_update;
                }
            }
        });
}