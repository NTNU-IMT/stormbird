// grid.wgsl will be prepended before this source during loading. `N` (total extended cell count
// for the coarsest level) and `NR_ITERATIONS` are injected as plain WGSL consts at
// shader-generation time, since both are fixed for the lifetime of the solver.
//
// Runs the entire bottom-of-V-cycle smoother (seed work buffer, set ghost cells on both buffers,
// then NR_ITERATIONS Jacobi+ghost sweeps) in a single dispatch of exactly one workgroup of size
// N, using workgroup-shared memory to ping-pong between iterations instead of round-tripping
// through the CPU for every iteration. This only runs when the coarsest grid is small enough to
// fit in one workgroup (checked on the Rust side before this shader is ever built).
//
// Per-cell classification (interior vs. core-ghost vs. unused corner/edge, plus the ghost
// neighbor offset/BC flag and interior RHS index) is precomputed on the CPU into `cells`, so this
// shader only has to branch on `cell.kind` rather than re-deriving axis/face membership itself.

const JACOBI_WEIGHT: f32 = 0.6666666667;

struct CellDescriptor {
    kind: u32,           // 0 = unused, 1 = interior, 2 = core ghost
    neighbor_delta: i32, // valid only when kind == 2
    zero_value: u32,     // valid only when kind == 2 (0 = ZeroGradient, 1 = ZeroValue)
    rhs_index: u32,      // valid only when kind == 1
}

@group(0) @binding(1) var<storage, read> cells: array<CellDescriptor, N>;
@group(0) @binding(2) var<storage, read> rhs: array<f32>;
@group(0) @binding(3) var<storage, read_write> x: array<f32>;

var<workgroup> buf: array<array<f32, N>, 2>;

fn jacobi_value(buf_idx: u32, idx: u32, rhs_index: u32) -> f32 {
    let off_diag =
          grid.inv_cell_length_squared.x * (buf[buf_idx][idx + grid.extended_stride.x] + buf[buf_idx][idx - grid.extended_stride.x])
        + grid.inv_cell_length_squared.y * (buf[buf_idx][idx + grid.extended_stride.y] + buf[buf_idx][idx - grid.extended_stride.y])
        + grid.inv_cell_length_squared.z * (buf[buf_idx][idx + 1u]                     + buf[buf_idx][idx - 1u]);

    let jacobi_update = (rhs[rhs_index] - off_diag) * grid.poisson_inv_diagonal;

    return (1.0 - JACOBI_WEIGHT) * buf[buf_idx][idx] + JACOBI_WEIGHT * jacobi_update;
}

fn ghost_value(buf_idx: u32, idx: u32, neighbor_delta: i32, zero_value: u32) -> f32 {
    let neighbor_idx = u32(i32(idx) + neighbor_delta);
    let neighbor_val = buf[buf_idx][neighbor_idx];

    return select(neighbor_val, -neighbor_val, zero_value == 1u);
}

@compute @workgroup_size(N)
fn main(@builtin(local_invocation_index) idx: u32) {
    let cell = cells[idx];
    let is_interior = cell.kind == 1u;
    let is_ghost = cell.kind == 2u;

    buf[0][idx] = x[idx];
    workgroupBarrier();

    buf[1][idx] = buf[0][idx];
    workgroupBarrier();

    if is_ghost {
        buf[0][idx] = ghost_value(0u, idx, cell.neighbor_delta, cell.zero_value);
    }
    workgroupBarrier();

    if is_ghost {
        buf[1][idx] = ghost_value(1u, idx, cell.neighbor_delta, cell.zero_value);
    }
    workgroupBarrier();

    for (var iteration = 0u; iteration < NR_ITERATIONS; iteration = iteration + 1u) {
        let cur = iteration % 2u;
        let nxt = 1u - cur;

        if is_interior {
            buf[nxt][idx] = jacobi_value(cur, idx, cell.rhs_index);
        }
        workgroupBarrier();

        if is_ghost {
            buf[nxt][idx] = ghost_value(nxt, idx, cell.neighbor_delta, cell.zero_value);
        }
        workgroupBarrier();
    }

    x[idx] = buf[NR_ITERATIONS % 2u][idx];
}
