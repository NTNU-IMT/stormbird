// Sets the ghost cells for all 6 boundary faces of a single buffer in one dispatch.
//
// Only the "core" of each face is written (the part where the two transverse indices are within
// the interior range) since that's the only part the Laplacian/Jacobi/restrict/prolongate
// stencils ever read — corner/edge ghost cells (where two or more indices are out of range) are
// never consumed downstream, so they're intentionally left untouched here.
//
// Because each face's core only reads its own axis-adjacent interior cells (never another face's
// ghost cells), all 6 faces are independent and can be resolved by a single flat dispatch instead
// of 6 serialized passes.

struct FaceDescriptor {
    axis_offset: u32,
    neighbor_delta: i32,
    shape: vec2<u32>,
    stride: vec2<u32>,
    zero_value: u32,
    start_offset: u32,
}

@group(0) @binding(0) var<storage, read> faces: array<FaceDescriptor, 6>;
@group(0) @binding(1) var<storage, read_write> p: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn set_ghost_cells(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;

    var face_index = 0u;
    var local_idx = idx;
    var matched = false;

    for (var f = 0u; f < 6u; f = f + 1u) {
        let count = faces[f].shape.x * faces[f].shape.y;
        if idx >= faces[f].start_offset && idx < faces[f].start_offset + count {
            face_index = f;
            local_idx = idx - faces[f].start_offset;
            matched = true;
            break;
        }
    }

    if !matched {
        return;
    }

    let face = faces[face_index];

    let i_outer = local_idx / face.shape.y;
    let i_inner = local_idx % face.shape.y;

    let flat_current = face.axis_offset + i_outer * face.stride.x + i_inner * face.stride.y;
    let flat_neighbor = u32(i32(flat_current) + face.neighbor_delta);

    let neighbor_val = p[flat_neighbor];

    if face.zero_value == 1u {
        p[flat_current] = -neighbor_val;
    } else {
        p[flat_current] = neighbor_val;
    }
}
