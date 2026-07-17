// grid.wgsl will be prepended before this source during loading. `N` (nr_interior_cells for the
// coarsest level) and `NR_ITERATIONS` are injected as plain WGSL consts at shader-generation
// time, since both are fixed for the lifetime of the solver. BC_X0..BC_Z1 are injected the same
// way as jacobi_shader.wgsl (see there for the boundary-folded stencil technique).
//
// Runs the entire bottom-of-V-cycle smoother (NR_ITERATIONS Jacobi sweeps) in a single dispatch
// of exactly one workgroup of size N, using workgroup-shared memory to ping-pong between
// iterations instead of round-tripping through the CPU for every iteration. Only runs when the
// coarsest grid is small enough to fit in one workgroup (checked on the Rust side before this
// shader is ever built). With no ghost cells, every one of the N threads is a genuine interior
// cell, so — unlike the padded-array version of this kernel — there's no separate cell
// classification needed: every thread runs the same boundary-folded stencil as the regular
// jacobi_shader.wgsl kernel, just reading/writing workgroup-shared memory instead of storage
// buffers, and only one workgroupBarrier() per iteration instead of two.

const JACOBI_WEIGHT: f32 = 0.6666666667;

@group(0) @binding(1) var<storage, read> rhs: array<f32>;
@group(0) @binding(2) var<storage, read_write> x: array<f32>;

var<workgroup> buf: array<array<f32, N>, 2>;

fn zero_value_flag(axis: u32, face: u32) -> u32 {
    if axis == 0u {
        return select(BC_X0, BC_X1, face == 1u);
    } else if axis == 1u {
        return select(BC_Y0, BC_Y1, face == 1u);
    } else {
        return select(BC_Z0, BC_Z1, face == 1u);
    }
}

/// Off-diagonal stencil sum, mirroring jacobi_shader.wgsl's `off_diagonal_sum`: a missing
/// neighbor on any axis is explicitly substituted by `buf[buf_idx][idx]` itself (signed per that
/// face's boundary condition) rather than folded into the diagonal.
fn off_diagonal_sum(buf_idx: u32, idx: u32, ii: u32, ji: u32, ki: u32) -> f32 {
    let nx = grid.interior_shape.x;
    let ny = grid.interior_shape.y;
    let nz = grid.interior_shape.z;

    var off_diag: f32 = 0.0;

    if ii > 0u {
        off_diag += grid.inv_cell_length_squared.x * buf[buf_idx][idx - grid.interior_stride.x];
    } else {
        off_diag += grid.inv_cell_length_squared.x * select(1.0, -1.0, zero_value_flag(0u, 0u) == 1u) * buf[buf_idx][idx];
    }
    if ii + 1u < nx {
        off_diag += grid.inv_cell_length_squared.x * buf[buf_idx][idx + grid.interior_stride.x];
    } else {
        off_diag += grid.inv_cell_length_squared.x * select(1.0, -1.0, zero_value_flag(0u, 1u) == 1u) * buf[buf_idx][idx];
    }

    if ji > 0u {
        off_diag += grid.inv_cell_length_squared.y * buf[buf_idx][idx - grid.interior_stride.y];
    } else {
        off_diag += grid.inv_cell_length_squared.y * select(1.0, -1.0, zero_value_flag(1u, 0u) == 1u) * buf[buf_idx][idx];
    }
    if ji + 1u < ny {
        off_diag += grid.inv_cell_length_squared.y * buf[buf_idx][idx + grid.interior_stride.y];
    } else {
        off_diag += grid.inv_cell_length_squared.y * select(1.0, -1.0, zero_value_flag(1u, 1u) == 1u) * buf[buf_idx][idx];
    }

    if ki > 0u {
        off_diag += grid.inv_cell_length_squared.z * buf[buf_idx][idx - 1u];
    } else {
        off_diag += grid.inv_cell_length_squared.z * select(1.0, -1.0, zero_value_flag(2u, 0u) == 1u) * buf[buf_idx][idx];
    }
    if ki + 1u < nz {
        off_diag += grid.inv_cell_length_squared.z * buf[buf_idx][idx + 1u];
    } else {
        off_diag += grid.inv_cell_length_squared.z * select(1.0, -1.0, zero_value_flag(2u, 1u) == 1u) * buf[buf_idx][idx];
    }

    return off_diag;
}

fn jacobi_value(buf_idx: u32, idx: u32, ii: u32, ji: u32, ki: u32) -> f32 {
    let off_diag = off_diagonal_sum(buf_idx, idx, ii, ji, ki);
    let update = (rhs[idx] - off_diag) * grid.poisson_inv_diagonal;

    return (1.0 - JACOBI_WEIGHT) * buf[buf_idx][idx] + JACOBI_WEIGHT * update;
}

@compute @workgroup_size(N)
fn main(@builtin(local_invocation_index) idx: u32) {
    // idx is exactly the interior flat index (0..N-1), matching Grid::flat_index_on_interior_grid.
    let ii = idx / grid.interior_stride.x;
    let ji = (idx % grid.interior_stride.x) / grid.interior_stride.y;
    let ki = idx % grid.interior_stride.y;

    buf[0][idx] = x[idx];
    workgroupBarrier();

    for (var iteration = 0u; iteration < NR_ITERATIONS; iteration = iteration + 1u) {
        let cur = iteration % 2u;
        let nxt = 1u - cur;

        buf[nxt][idx] = jacobi_value(cur, idx, ii, ji, ki);
        workgroupBarrier();
    }

    x[idx] = buf[NR_ITERATIONS % 2u][idx];
}
