struct GridUniforms {
    interior_shape_x : u32,
    interior_shape_y : u32,
    interior_shape_z : u32,
    extended_stride_x: u32,
    extended_stride_y: u32,
    inv_dx2          : f32,
    inv_dy2          : f32,
    inv_dz2          : f32,
    poisson_inv_diagonal : f32,
}

@group(0) @binding(0) var<uniform>            grid     : GridUniforms;
@group(0) @binding(1) var<storage, read>      current  : array<f32>;  // extended grid
@group(0) @binding(2) var<storage, read>      rhs      : array<f32>;  // interior grid
@group(0) @binding(3) var<storage, read_write> new_sol : array<f32>;  // extended grid

const WG: u32 = 4u;
const JACOBI_WEIGHT: f32 = 0.6666666667;

@compute @workgroup_size(WG, WG, WG)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ii = gid.x;
    let ji = gid.y;
    let ki = gid.z;

    if ii >= grid.interior_shape_x ||
       ji >= grid.interior_shape_y ||
       ki >= grid.interior_shape_z {
        return;
    }

    // Extended-grid index (ghost cell offset = +1 in each dim)
    let i = ii + 1u;
    let j = ji + 1u;
    let k = ki + 1u;
    let idx = i * grid.extended_stride_x + j * grid.extended_stride_y + k;

    // Interior-grid index for RHS (z-major, no ghost cells)
    let idx_int = ii * (grid.interior_shape_y * grid.interior_shape_z)
                + ji *  grid.interior_shape_z
                + ki;

    let off_diag =
          grid.inv_dx2 * (current[idx + grid.extended_stride_x] + current[idx - grid.extended_stride_x])
        + grid.inv_dy2 * (current[idx + grid.extended_stride_y] + current[idx - grid.extended_stride_y])
        + grid.inv_dz2 * (current[idx + 1u]                     + current[idx - 1u]);

    let jacobi_update = (rhs[idx_int] - off_diag) * grid.poisson_inv_diagonal;

    new_sol[idx] = (1.0 - JACOBI_WEIGHT) * current[idx] + JACOBI_WEIGHT * jacobi_update;
}