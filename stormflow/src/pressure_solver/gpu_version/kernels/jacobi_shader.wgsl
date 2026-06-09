// The grid.wgsl shader will be prepended before the rest of the source during loading

@group(0) @binding(1) var<storage, read> current: array<f32>;
@group(0) @binding(2) var<storage, read> rhs: array<f32>; 
@group(0) @binding(3) var<storage, read_write> new_sol: array<f32>;

const WG: u32 = 4u;
const JACOBI_WEIGHT: f32 = 0.6666666667;

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

    // Extended-grid index (ghost cell offset = +1 in each dim)
    let i = ii + 1u;
    let j = ji + 1u;
    let k = ki + 1u;
    let idx = i * grid.extended_stride.x + j * grid.extended_stride.y + k;

    // Interior-grid index for RHS (z-major, no ghost cells)
    let idx_int = ii * (grid.interior_shape.y * grid.interior_shape.z)
                + ji *  grid.interior_shape.z
                + ki;

    let off_diag =
          grid.inv_dx2 * (current[idx + grid.extended_stride.x] + current[idx - grid.extended_stride.x])
        + grid.inv_dy2 * (current[idx + grid.extended_stride.y] + current[idx - grid.extended_stride.y])
        + grid.inv_dz2 * (current[idx + 1u]                     + current[idx - 1u]);

    let jacobi_update = (rhs[idx_int] - off_diag) * grid.poisson_inv_diagonal;

    new_sol[idx] = (1.0 - JACOBI_WEIGHT) * current[idx] + JACOBI_WEIGHT * jacobi_update;
}
