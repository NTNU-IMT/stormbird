// GPU version of `VelocityBoundaryConditions::set_ghost_cells`, for one boundary face per dispatch.
// Each invocation handles one column through the face, and fills all INTERIOR_OFFSET ghost layers
// of that column. Ghost layer `l` (0 = nearest the interior) is paired with the interior cell
// `2 * l + 1` cells away, the same pairing as `BoundaryFace`, so a ghost cell never reads another
// ghost cell of the same face. The normal velocity component of slip walls is mirrored with the
// opposite sign across the wall, which, due to the staggering, uses a different pairing, and the
// inlet/outlet condition checks the flow direction at the interior cell adjacent to the boundary
// for all layers. See `VelocityBoundaryConditions::set_ghost_cells_kernel` for the details. The
// faces must be dispatched in the same order as on the CPU (axis-major, then face), since edge
// and corner cells are written by more than one face.

// Matches `GpuFaceParams` on the Rust side.
struct FaceParams {
    axis: u32,
    // 0 for the face at the start of the axis, 1 for the face at the end
    face: u32,
    // 0 = ZeroGradient, 1 = InletOutlet, 2 = SlipWall
    condition: u32,
    _pad: u32,
}

const ZERO_GRADIENT: u32 = 0u;
const INLET_OUTLET: u32 = 1u;
const SLIP_WALL: u32 = 2u;

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<uniform> face_params: FaceParams;
// The staggered inlet velocity for each extended cell layer along UP_AXIS. See
// `VelocityBoundaryConditions::inlet_velocity_profile`.
@group(0) @binding(2) var<storage, read> inlet_velocity_profile: array<f32>;
@group(0) @binding(3) var<storage, read_write> field: array<f32>;

@compute @workgroup_size(WG_PLANE, WG_PLANE, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let axis = face_params.axis;

    // The two remaining axes of the face. `inner_axis` has the smallest stride.
    var outer_axis: u32;
    var inner_axis: u32;

    if axis == 0u {
        outer_axis = 1u;
        inner_axis = 2u;
    } else if axis == 1u {
        outer_axis = 0u;
        inner_axis = 2u;
    } else {
        outer_axis = 0u;
        inner_axis = 1u;
    }

    let shape = grid.extended_shape.xyz;
    let stride = grid.extended_stride.xyz;

    if gid.x >= shape[inner_axis] || gid.y >= shape[outer_axis] {
        return;
    }

    let column_offset = gid.y * stride[outer_axis] + gid.x * stride[inner_axis];

    let axis_length = shape[axis];
    let axis_stride = stride[axis];

    let at_min_face = face_params.face == 0u;

    for (var layer: u32 = 0u; layer < INTERIOR_OFFSET; layer = layer + 1u) {
        var ghost: u32;
        var neighbor: u32;
        var adjacent: u32;

        if at_min_face {
            ghost = INTERIOR_OFFSET - 1u - layer;
            neighbor = ghost + 2u * layer + 1u;
            adjacent = ghost + layer + 1u;
        } else {
            ghost = axis_length - INTERIOR_OFFSET + layer;
            neighbor = ghost - (2u * layer + 1u);
            adjacent = ghost - (layer + 1u);
        }

        let flat_current = column_offset + ghost * axis_stride;
        let flat_neighbor = column_offset + neighbor * axis_stride;

        var new_value = vec3<f32>(
            field[3u * flat_neighbor],
            field[3u * flat_neighbor + 1u],
            field[3u * flat_neighbor + 2u],
        );

        if face_params.condition == SLIP_WALL {
            // Opposite sign mirror of the normal component across the wall. At the start of the
            // axis, the normal face of layer 0 is on the wall itself.
            if at_min_face && layer == 0u {
                new_value[axis] = 0.0;
            } else {
                var mirror: u32;

                if at_min_face {
                    mirror = ghost + 2u * layer;
                } else {
                    mirror = ghost - (2u * layer + 2u);
                }

                new_value[axis] = -field[3u * (column_offset + mirror * axis_stride) + axis];
            }
        } else if face_params.condition == INLET_OUTLET {
            let adjacent_axis_flow = field[3u * (column_offset + adjacent * axis_stride) + axis];

            var inflow: bool;

            if at_min_face {
                inflow = adjacent_axis_flow > 0.0;
            } else {
                inflow = adjacent_axis_flow < 0.0;
            }

            if inflow {
                let i_up = (flat_current / stride[UP_AXIS]) % shape[UP_AXIS];

                new_value = vec3<f32>(
                    inlet_velocity_profile[3u * i_up],
                    inlet_velocity_profile[3u * i_up + 1u],
                    inlet_velocity_profile[3u * i_up + 2u],
                );
            }
        }

        field[3u * flat_current] = new_value.x;
        field[3u * flat_current + 1u] = new_value.y;
        field[3u * flat_current + 2u] = new_value.z;

        // At the end of the axis, the normal face on the wall belongs to the last interior cell
        if face_params.condition == SLIP_WALL && !at_min_face && layer == 0u {
            field[3u * (flat_current - axis_stride) + axis] = 0.0;
        }
    }
}
