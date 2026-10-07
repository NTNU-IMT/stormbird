// GPU version of `pressure_rhs_kernel`: the right hand side of the pressure equation, written to an
// array on the **interior** grid. Dispatched over the interior cells, with gid.x along the
// contiguous z-axis.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<uniform> params: VelocityParams;
@group(0) @binding(2) var<storage, read> velocity_star: array<f32>;
@group(0) @binding(3) var<storage, read_write> rhs: array<f32>;

fn vel_star(i: u32, component: u32) -> f32 {
    return velocity_star[3u * i + component];
}

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

    var divergence: f32 = 0.0;

    for (var axis: u32 = 0u; axis < 3u; axis = axis + 1u) {
        let stride = grid.extended_stride[axis];

        let i_n = i_0 - stride;
        let i_p = i_0 + stride;
        let i_n2 = i_n - stride;

        divergence += (
            27.0 * (vel_star(i_0, axis) - vel_star(i_n, axis)) -
            (vel_star(i_p, axis) - vel_star(i_n2, axis))
        ) * grid.inv_cell_length[axis] * (1.0 / 24.0);
    }

    let i_interior = ii * grid.interior_stride.x + ji * grid.interior_stride.y + ki;

    rhs[i_interior] = divergence * params.rhs_scale;
}
