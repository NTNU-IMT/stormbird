// The grid.wgsl shader will be prepended before the rest of the source during loading.
// BC_X0/BC_X1/BC_Y0/BC_Y1/BC_Z0/BC_Z1 (0 = ZeroGradient, 1 = ZeroValue) are injected as plain
// WGSL consts at shader-generation time, since the boundary condition configuration is fixed for
// the lifetime of the solver and doesn't depend on grid resolution, so this pipeline is shared
// across all multigrid levels.
//
// No ghost cells: `current`/`rhs`/`new_sol` are all sized to the INTERIOR grid only, dispatched
// over interior_shape. Boundary conditions are folded directly into the stencil instead of being
// read from a padded array: for a cell missing a real neighbor on one side, a ZeroGradient face
// substitutes `current[idx]` in place of the missing `current[i-1]`/`current[i+1]` read (mirrors
// the cell's own — old, not yet updated this pass — value), and a ZeroValue face substitutes
// `-current[idx]`. This is an *explicit* substitution using the already-known `current[idx]`
// (matching the lagged ghost cell a separate ghost-cell pass would have produced from the
// *previous* Jacobi sweep), not an implicit one — it must be added to `off_diag`, not folded into
// the diagonal coefficient, since it isn't solving for the new value self-referentially.

@group(0) @binding(1) var<storage, read> current: array<f32>;
@group(0) @binding(2) var<storage, read> rhs: array<f32>;
@group(0) @binding(3) var<storage, read_write> new_sol: array<f32>;

const WG: u32 = 4u;
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

/// Off-diagonal stencil sum for the interior cell at (ii,ji,ki)/idx. For every axis where a real
/// neighbor exists it's read directly; where it doesn't (a boundary-adjacent cell), the missing
/// read is replaced by `current[idx]` itself, signed per the corresponding face's boundary
/// condition (see the file header for why this is an explicit substitution, not a diagonal fold).
fn off_diagonal_sum(idx: u32, ii: u32, ji: u32, ki: u32) -> f32 {
    let nx = grid.interior_shape.x;
    let ny = grid.interior_shape.y;
    let nz = grid.interior_shape.z;

    var off_diag: f32 = 0.0;

    if ii > 0u {
        off_diag += grid.inv_cell_length_squared.x * current[idx - grid.interior_stride.x];
    } else {
        off_diag += grid.inv_cell_length_squared.x * select(1.0, -1.0, zero_value_flag(0u, 0u) == 1u) * current[idx];
    }
    if ii + 1u < nx {
        off_diag += grid.inv_cell_length_squared.x * current[idx + grid.interior_stride.x];
    } else {
        off_diag += grid.inv_cell_length_squared.x * select(1.0, -1.0, zero_value_flag(0u, 1u) == 1u) * current[idx];
    }

    if ji > 0u {
        off_diag += grid.inv_cell_length_squared.y * current[idx - grid.interior_stride.y];
    } else {
        off_diag += grid.inv_cell_length_squared.y * select(1.0, -1.0, zero_value_flag(1u, 0u) == 1u) * current[idx];
    }
    if ji + 1u < ny {
        off_diag += grid.inv_cell_length_squared.y * current[idx + grid.interior_stride.y];
    } else {
        off_diag += grid.inv_cell_length_squared.y * select(1.0, -1.0, zero_value_flag(1u, 1u) == 1u) * current[idx];
    }

    if ki > 0u {
        off_diag += grid.inv_cell_length_squared.z * current[idx - 1u];
    } else {
        off_diag += grid.inv_cell_length_squared.z * select(1.0, -1.0, zero_value_flag(2u, 0u) == 1u) * current[idx];
    }
    if ki + 1u < nz {
        off_diag += grid.inv_cell_length_squared.z * current[idx + 1u];
    } else {
        off_diag += grid.inv_cell_length_squared.z * select(1.0, -1.0, zero_value_flag(2u, 1u) == 1u) * current[idx];
    }

    return off_diag;
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
    let jacobi_update = (rhs[idx] - off_diag) * grid.poisson_inv_diagonal;

    new_sol[idx] = (1.0 - JACOBI_WEIGHT) * current[idx] + JACOBI_WEIGHT * jacobi_update;
}
