// GPU version of the ghost cell update in `turbulence_solver::boundary_conditions`, for one
// boundary face per dispatch. Each invocation handles one column through the face, and fills all
// INTERIOR_OFFSET ghost layers of all `nr_fields` fields of that column. Ghost layer `l` (0 =
// nearest the interior) is paired with the interior cell `2 * l + 1` cells away, the same pairing
// as `BoundaryFace`. The faces must be dispatched in the same order as on the CPU (axis-major,
// then face), since edge and corner cells are written by more than one face.

// Matches `GpuScalarFaceParams` on the Rust side.
struct FaceParams {
    axis: u32,
    // 0 for the face at the start of the axis, 1 for the face at the end
    face: u32,
    // 0 = ZeroGradient, 1 = InletOutlet
    condition: u32,
    nr_fields: u32,
}

const INLET_OUTLET: u32 = 1u;

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<uniform> face_params: FaceParams;
// The inlet values for each extended cell layer along UP_AXIS, layer-major. See
// `TurbulenceBoundaryConditions::inlet_profile`.
@group(0) @binding(2) var<storage, read> inlet_profile: array<f32>;
@group(0) @binding(3) var<storage, read> velocity: array<f32>;
@group(0) @binding(4) var<storage, read_write> field: array<f32>;

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

        var inflow = false;

        if face_params.condition == INLET_OUTLET {
            let adjacent_axis_flow = velocity[3u * (column_offset + adjacent * axis_stride) + axis];

            if at_min_face {
                inflow = adjacent_axis_flow > 0.0;
            } else {
                inflow = adjacent_axis_flow < 0.0;
            }
        }

        let i_up = (flat_current / stride[UP_AXIS]) % shape[UP_AXIS];

        for (var f: u32 = 0u; f < face_params.nr_fields; f = f + 1u) {
            let offset = f * N_EXTENDED_CELLS;

            if inflow {
                field[offset + flat_current] = inlet_profile[i_up * face_params.nr_fields + f];
            } else {
                field[offset + flat_current] = field[offset + flat_neighbor];
            }
        }
    }
}
