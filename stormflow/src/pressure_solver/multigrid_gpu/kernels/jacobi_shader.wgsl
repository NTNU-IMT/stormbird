// gpu_grid.wgsl and jacobi_common.wgsl are prepended before this during loading (see
// jacobi_shader.rs) — off_diagonal_sum/jacobi_relaxed_update and friends live there, shared with
// jacobi_slip_shader.wgsl. BC_X0/BC_X1/BC_Y0/BC_Y1/BC_Z0/BC_Z1 (0 = ZeroGradient, 1 = ZeroValue)
// are injected as plain WGSL consts at shader-generation time, since the boundary condition
// configuration is fixed for the lifetime of the solver and doesn't depend on grid resolution, so
// this pipeline is shared across all multigrid levels.
//
// No ghost cells: `current`/`rhs`/`new_sol` are all sized to the INTERIOR grid only, dispatched
// over interior_shape.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<storage, read> current: array<f32>;
@group(0) @binding(2) var<storage, read> rhs: array<f32>;
@group(0) @binding(3) var<storage, read_write> new_sol: array<f32>;

const WG: u32 = 8u;

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

    new_sol[idx] = jacobi_relaxed_update(idx, ii, ji, ki);
}
