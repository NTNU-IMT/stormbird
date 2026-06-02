// The boundary_face.wgsl shader will be prepended before the rest of the source during loading

@group(0) @binding(1) var<storage, read_write> p: array<f32>;
@group(0) @binding(2) var<uniform> zero_value: u32; // 0 = ZeroGradient, 1 = ZeroValue

@compute @workgroup_size(64, 1, 1)
fn set_ghost_cells(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let total = boundary_face.shape.x * boundary_face.shape.y;

    if idx >= total {
        return;
    }

    let i_outer = idx / boundary_face.shape.y;
    let i_inner = idx % boundary_face.shape.y;

    let flat_current = boundary_face.axis_offset
        + i_outer * boundary_face.stride.y
        + i_inner * boundary_face.stride.x;

    let flat_neighbor = u32(i32(flat_current) + boundary_face.neighbor_delta);

    let neighbor_val = p[flat_neighbor];

    if zero_value == 1u {
        p[flat_current] = -neighbor_val;
    } else {
        p[flat_current] = neighbor_val;
    }
}
