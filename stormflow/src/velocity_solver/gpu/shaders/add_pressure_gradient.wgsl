// GPU version of `add_pressure_gradient_kernel`. Dispatched over the interior cells, with gid.x
// along the contiguous z-axis.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<uniform> params: VelocityParams;
@group(0) @binding(2) var<storage, read> pressure: array<f32>;
@group(0) @binding(3) var<storage, read> velocity_star: array<f32>;
@group(0) @binding(4) var<storage, read_write> velocity: array<f32>;

@compute @workgroup_size(WG_X, WG_Y, WG_Z)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ki = gid.x;
    let ji = gid.y;
    let ii = gid.z;

    if ii >= grid.interior_shape.x || ji >= grid.interior_shape.y || ki >= grid.interior_shape.z {
        return;
    }

    let i_0 = (ii + INTERIOR_OFFSET) * grid.extended_stride.x +
              (ji + INTERIOR_OFFSET) * grid.extended_stride.y +
              (ki + INTERIOR_OFFSET);

    for (var axis: u32 = 0u; axis < 3u; axis = axis + 1u) {
        let stride = grid.extended_stride[axis];

        let i_n = i_0 - stride;
        let i_p = i_0 + stride;
        let i_p2 = i_p + stride;

        let dp_dx = (
            27.0 * (pressure[i_p] - pressure[i_0]) -
            (pressure[i_p2] - pressure[i_n])
        ) * grid.inv_cell_length[axis] * (1.0 / 24.0);

        velocity[3u * i_0 + axis] = velocity_star[3u * i_0 + axis] - params.gradient_scale * dp_dx;
    }
}
