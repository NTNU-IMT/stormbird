// gpu_grid.wgsl will be prepended before the rest of the source during loading.
// BC_X0/BC_X1/BC_Y0/BC_Y1/BC_Z0/BC_Z1 (0 = ZeroGradient, 1 = ZeroValue) are injected as plain
// WGSL consts at shader-generation time, since the boundary condition configuration is fixed for
// the lifetime of the solver and doesn't depend on grid resolution, so this pipeline is shared
// across all multigrid levels.
//
// No ghost cells: `current`/`rhs`/`new_sol` are all sized to the INTERIOR grid only, dispatched
// over interior_shape. Boundary conditions are folded directly into the stencil instead of being
// read from a padded array — see `axis_off_diagonal`'s doc comment for the 4th order substitution
// rule. This is an *explicit* substitution using the already-known `current` values (matching the
// lagged ghost cells a separate ghost-cell pass would have produced from the *previous* Jacobi
// sweep), not an implicit one — it must be added to `off_diag`, not folded into the diagonal
// coefficient, since it isn't solving for the new value self-referentially.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<storage, read> current: array<f32>;
@group(0) @binding(2) var<storage, read> rhs: array<f32>;
@group(0) @binding(3) var<storage, read_write> new_sol: array<f32>;

const WG: u32 = 8u;
const JACOBI_WEIGHT: f32 = 0.6666666667;

fn zero_value_flag(axis: u32, face: u32) -> u32 {
    if axis == 0u {
        return select(BC_X0, BC_X1, face == 1u);
    } else if axis == 1u {
        return select(BC_Y0, BC_Y1, face == 1u);
    } else {
        return select(BC_Z0, BC_Z1, face == 1u);
    }
}

/// One axis' contribution to the 4th order accurate (5-point) off-diagonal stencil sum, for the
/// interior cell at `position` (0-indexed from this axis' `0`-face) out of `count` interior
/// cells, `stride` apart in `current`.
///
/// The 5-point stencil `[-1, 16, -30, 16, -1]/(12h^2)` needs 2 neighbors on each side, so cells
/// within 2 of a boundary are missing one or both of them. Each missing read is substituted by
/// the interior value it mirrors across that boundary face (signed per the face's boundary
/// condition): the layer nearest the interior mirrors the boundary-adjacent cell itself
/// (`current[idx]`), the next layer out mirrors the *next* interior cell in
/// (`current[idx +/- stride]`) — matching `PressureBoundaryConditions::set_ghost_cells_kernel`'s
/// mirror-pairing rule on the CPU side. Must stay in sync with
/// `multigrid_cpu::kernels::jacobi::axis_off_diagonal_sum`.
fn axis_off_diagonal(idx: u32, stride: u32, position: u32, count: u32, sign_min: f32, sign_max: f32, inv_dx2: f32) -> f32 {
    var u_m1: f32;
    if position >= 1u {
        u_m1 = current[idx - stride];
    } else {
        u_m1 = sign_min * current[idx];
    }

    var u_m2: f32;
    if position >= 2u {
        u_m2 = current[idx - 2u * stride];
    } else if position == 1u {
        u_m2 = sign_min * current[idx - stride];
    } else {
        u_m2 = sign_min * current[idx + stride];
    }

    var u_p1: f32;
    if position + 1u < count {
        u_p1 = current[idx + stride];
    } else {
        u_p1 = sign_max * current[idx];
    }

    var u_p2: f32;
    if position + 2u < count {
        u_p2 = current[idx + 2u * stride];
    } else if position + 1u < count {
        u_p2 = sign_max * current[idx + stride];
    } else {
        u_p2 = sign_max * current[idx - stride];
    }

    return inv_dx2 * ((4.0 / 3.0) * (u_m1 + u_p1) - (1.0 / 12.0) * (u_m2 + u_p2));
}

/// Off-diagonal stencil sum for the interior cell at (ii,ji,ki)/idx — see `axis_off_diagonal` for
/// the per-axis boundary-folding logic.
fn off_diagonal_sum(idx: u32, ii: u32, ji: u32, ki: u32) -> f32 {
    let nx = grid.interior_shape.x;
    let ny = grid.interior_shape.y;
    let nz = grid.interior_shape.z;

    let sign_x0 = select(1.0, -1.0, zero_value_flag(0u, 0u) == 1u);
    let sign_x1 = select(1.0, -1.0, zero_value_flag(0u, 1u) == 1u);
    let sign_y0 = select(1.0, -1.0, zero_value_flag(1u, 0u) == 1u);
    let sign_y1 = select(1.0, -1.0, zero_value_flag(1u, 1u) == 1u);
    let sign_z0 = select(1.0, -1.0, zero_value_flag(2u, 0u) == 1u);
    let sign_z1 = select(1.0, -1.0, zero_value_flag(2u, 1u) == 1u);

    return axis_off_diagonal(idx, grid.interior_stride.x, ii, nx, sign_x0, sign_x1, grid.inv_cell_length_squared.x)
         + axis_off_diagonal(idx, grid.interior_stride.y, ji, ny, sign_y0, sign_y1, grid.inv_cell_length_squared.y)
         + axis_off_diagonal(idx, 1u, ki, nz, sign_z0, sign_z1, grid.inv_cell_length_squared.z);
}

@compute @workgroup_size(WG, WG, WG)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ii = gid.x;
    let ji = gid.y;
    let ki = gid.z;

    if ii >= grid.interior_shape.x ||
       ji >= grid.interior_shape.y ||
       ki >= grid.interior_shape.z {
        return;
    }

    let idx = ii * grid.interior_stride.x + ji * grid.interior_stride.y + ki;

    let off_diag = off_diagonal_sum(idx, ii, ji, ki);
    let jacobi_update = (rhs[idx] - off_diag) * grid.poisson_inv_diagonal4;

    new_sol[idx] = (1.0 - JACOBI_WEIGHT) * current[idx] + JACOBI_WEIGHT * jacobi_update;
}
