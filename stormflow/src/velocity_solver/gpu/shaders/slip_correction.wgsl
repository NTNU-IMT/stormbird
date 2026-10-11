// GPU version of the two-phase slip mirror correction in
// `VelocitySolverCPU::correct_velocities_for_slip_geometry`:
//
// - `compute`: evaluates `correct_slip_mirror_entry_kernel` for every entry, reading only the
//   uncorrected field, and stores the results in the compact `new_values` array.
// - `scatter`: writes `new_values` back into the field, in a separate dispatch.
//
// Separating the two means all entries sample the same uncorrected field, independent of the order
// the entries are processed in. SLIP_N is the number of interpolation weights per axis (2 for
// trilinear, 4 for tricubic), shared by all entries.

// Matches `GpuSlipEntry` on the Rust side.
struct SlipEntry {
    // Index into the flat vector field: 3 * cell_index + axis_index
    field_index: u32,
    mu: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
    // Base flat extended index of the interpolation stencil of each velocity component
    base_index_u: u32,
    base_index_v: u32,
    base_index_w: u32,
}

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<storage, read> entries: array<SlipEntry>;
// Per entry: for each velocity component, SLIP_N weights for each of the 3 axes, i.e.,
// [component0: [axis0, axis1, axis2], component1: [...], component2: [...]]
@group(0) @binding(2) var<storage, read> weights: array<f32>;
@group(0) @binding(3) var<storage, read_write> field: array<f32>;
@group(0) @binding(4) var<storage, read_write> new_values: array<f32>;

/// Same separable tensor-product gather as `TrilinearStencil`/`TricubicStencil::sample_component`
fn sample_component(e: u32, component: u32, base: u32) -> f32 {
    let w_offset = (e * 3u + component) * 3u * SLIP_N;

    var sum: f32 = 0.0;

    for (var a: u32 = 0u; a < SLIP_N; a = a + 1u) {
        let wa = weights[w_offset + a];

        for (var b: u32 = 0u; b < SLIP_N; b = b + 1u) {
            let wb = weights[w_offset + SLIP_N + b];

            for (var c: u32 = 0u; c < SLIP_N; c = c + 1u) {
                let wc = weights[w_offset + 2u * SLIP_N + c];

                let idx = base
                    + a * grid.extended_stride.x
                    + b * grid.extended_stride.y
                    + c * grid.extended_stride.z;

                sum += wa * wb * wc * field[3u * idx + component];
            }
        }
    }

    return sum;
}

@compute @workgroup_size(WG_1D)
fn compute(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&entries) {
        return;
    }

    let item = entries[e];

    let v_image = vec3<f32>(
        sample_component(e, 0u, item.base_index_u),
        sample_component(e, 1u, item.base_index_v),
        sample_component(e, 2u, item.base_index_w),
    );

    let normal = vec3<f32>(item.normal_x, item.normal_y, item.normal_z);

    let v_image_dot_normal = v_image.x * normal.x + v_image.y * normal.y + v_image.z * normal.z;

    let axis = item.field_index % 3u;

    let ghost_velocity = v_image[axis] - 2.0 * v_image_dot_normal * normal[axis];

    let current = field[item.field_index];

    new_values[e] = item.mu * current + (1.0 - item.mu) * ghost_velocity;
}

@compute @workgroup_size(WG_1D)
fn scatter(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&entries) {
        return;
    }

    field[entries[e].field_index] = new_values[e];
}
