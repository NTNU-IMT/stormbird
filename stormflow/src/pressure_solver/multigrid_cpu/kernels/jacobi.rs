use crate::{
    grid::Grid,
    pressure_solver::boundary_conditions::PressureBoundaryConditions,
    pressure_solver::multigrid_cpu::slip_pressure_stencils::SlipPressureStencils
};

use stormath::type_aliases::Float;

pub const JACOBI_WEIGHT: Float = 0.666_666_7;

/// Under-relaxation factor for the slip-pressure zero-gradient correction blend (see
/// `jacobi_kernel_with_slip_correction`), analogous to `JACOBI_WEIGHT` for the plain Jacobi update.
/// The tricubic interpolation used to sample the mirrored image point has non-convex weights (some
/// negative; the per-axis weights sum to 1 but sum in absolute value to noticeably more than 1,
/// unlike trilinear's always-non-negative, non-expansive weights), so it can mildly amplify
/// oscillatory error. That's harmless applied once every several Jacobi sweeps, but folding it into
/// *every* sweep turned that occasional overshoot into part of the iteration matrix itself and
/// produced slow, compounding growth instead of convergence. Damping how far each application
/// actually moves a corrected cell keeps most of the per-iteration correction benefit while
/// limiting how much amplification can accumulate.
pub const SLIP_CORRECTION_RELAXATION: Float = 0.5;

#[inline]
pub(crate) fn boundary_sign(boundary_conditions: &PressureBoundaryConditions, axis: usize, face: usize) -> Float {
    if boundary_conditions.condition(axis, face).as_gpu_flag() == 1 { -1.0 } else { 1.0 }
}

/// One axis' contribution to the 4th order accurate (5-point) off-diagonal stencil sum, for the
/// interior cell at `position` (0-indexed from this axis' `0`-face) out of `count` interior
/// cells, `stride` apart in `current`.
///
/// The 5-point stencil `[-1, 16, -30, 16, -1]/(12h²)` needs 2 neighbors on each side, so cells
/// within 2 of a boundary are missing one or both of them. Each missing read is substituted by
/// the interior value it mirrors across that boundary face (signed per the face's boundary
/// condition), exactly as `PressureBoundaryConditions::set_ghost_cells_kernel` fills the
/// corresponding ghost layer: the layer nearest the interior mirrors the boundary-adjacent cell
/// itself (`current[idx]`), the next layer out mirrors the *next* interior cell in
/// (`current[idx ± stride]`) — see `grid::boundary_face::BoundaryFace::new`'s doc comment for the
/// same mirror-pairing rule. This is the "boundary-folded stencil" technique, mathematically
/// equivalent to ghost cells but without maintaining a padded array. See `multigrid_gpu`'s
/// `jacobi_shader.wgsl` for the same technique on the GPU side; the two must stay in sync.
///
/// This is an *explicit* substitution using the already-known `current` values (matching the
/// ghost cells a separate pass would have produced from the *previous* Jacobi sweep), not an
/// implicit one — it belongs in the off-diagonal sum, not folded into the diagonal coefficient,
/// since it isn't solving for the new value self-referentially.
#[inline(always)]
fn axis_off_diagonal_sum(
    current: &[Float],
    idx: usize,
    stride: usize,
    position: usize,
    count: usize,
    sign_min: Float,
    sign_max: Float,
    inv_cell_length_squared: Float,
) -> Float {
    let u_m1 = if position >= 1 {
        current[idx - stride]
    } else {
        sign_min * current[idx]
    };

    let u_m2 = if position >= 2 {
        current[idx - 2 * stride]
    } else if position == 1 {
        sign_min * current[idx - stride]
    } else {
        sign_min * current[idx + stride]
    };

    let u_p1 = if position + 1 < count {
        current[idx + stride]
    } else {
        sign_max * current[idx]
    };

    let u_p2 = if position + 2 < count {
        current[idx + 2 * stride]
    } else if position + 1 < count {
        sign_max * current[idx + stride]
    } else {
        sign_max * current[idx - stride]
    };

    inv_cell_length_squared * ((4.0 / 3.0) * (u_m1 + u_p1) - (1.0 / 12.0) * (u_m2 + u_p2))
}

/// Off-diagonal stencil sum for the interior cell at (ii,ji,ki)/idx — see
/// `axis_off_diagonal_sum` for the per-axis boundary-folding logic.
#[inline(always)]
pub fn off_diagonal_sum(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    current: &[Float],
    idx: usize,
    indices: [usize; 3]
) -> Float {
    let [ii, ji, ki] = indices;
    let [nx, ny, nz] = grid.interior_shape;
    let [sx, sy, sz] = grid.interior_stride;

    axis_off_diagonal_sum(
        current, idx, sx, ii, nx,
        boundary_sign(boundary_conditions, 0, 0), boundary_sign(boundary_conditions, 0, 1),
        grid.inv_cell_length_squared[0]
    ) +
    axis_off_diagonal_sum(
        current, idx, sy, ji, ny,
        boundary_sign(boundary_conditions, 1, 0), boundary_sign(boundary_conditions, 1, 1),
        grid.inv_cell_length_squared[1]
    ) +
    axis_off_diagonal_sum(
        current, idx, sz, ki, nz,
        boundary_sign(boundary_conditions, 2, 0), boundary_sign(boundary_conditions, 2, 1),
        grid.inv_cell_length_squared[2]
    )
}

/// Performs a single Jacobi update for the Poisson equation at interior cell (ii,ji,ki)/idx.
/// `rhs`/`current` are sized to the **interior** grid — there are no ghost cells; boundary
/// conditions are folded directly into the stencil via `off_diagonal_sum`.
///
/// Reads from the `current` buffer; the result is meant to be written into a separate `new`
/// buffer by the caller (see `MultigridCPU::poisson_jacobi_smoother`), which is what lets
/// consecutive iterations alternate buffers without a copy.
#[inline(always)]
pub fn jacobi_kernel(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    rhs: &[Float],
    current: &[Float],
    idx: usize,
    indices: [usize; 3]
) -> Float {
    let off_diag = off_diagonal_sum(grid, boundary_conditions, current, idx, indices);
    let jacobi_update = (rhs[idx] - off_diag) * grid.poisson_inv_diagonal4;

    (1.0 - JACOBI_WEIGHT) * current[idx] + JACOBI_WEIGHT * jacobi_update
}

/// Same Jacobi update as `jacobi_kernel`, with the pressure zero-gradient (Neumann) slip-wall
/// correction folded directly into the same pass instead of applied as a separate post-hoc step:
/// `slip_pressure_stencils.cell_lookup[idx]` is `-1` for the overwhelming majority of cells (a
/// single cheap, sequentially-accessed, branch-predictable array read — see
/// `SlipPressureStencils::cell_lookup`'s doc comment), in which case this is exactly
/// `jacobi_kernel`. Only for the small, precomputed band of corrected cells does it look up that
/// cell's `SlipPressureEntry` and blend in the mirrored image point, sampled from the same
/// `current` read buffer the plain Jacobi update already reads its neighbors from — reusing the
/// smoother's existing double-buffer swap for the "read old, write new" isolation a separate
/// correction pass would otherwise need a full extra snapshot copy for.
#[inline(always)]
pub fn jacobi_kernel_with_slip_correction(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    rhs: &[Float],
    current: &[Float],
    idx: usize,
    indices: [usize; 3],
    slip_pressure_stencils: &SlipPressureStencils
) -> Float {
    let relaxed = jacobi_kernel(grid, boundary_conditions, rhs, current, idx, indices);

    let lookup_index = slip_pressure_stencils.cell_lookup[idx];

    if lookup_index < 0 {
        return relaxed;
    }

    let entry = &slip_pressure_stencils.entries[lookup_index as usize];
    let p_image = entry.stencil.sample_scalar(current, grid.interior_stride);

    let corrected_target = entry.mu * relaxed + (1.0 - entry.mu) * p_image;

    // Under-relax the correction itself (see `SLIP_CORRECTION_RELAXATION`'s doc comment) — move
    // only partway from the cell's own previous value toward the blended target, rather than
    // fully committing to it every single sweep.
    (1.0 - SLIP_CORRECTION_RELAXATION) * current[idx] + SLIP_CORRECTION_RELAXATION * corrected_target
}