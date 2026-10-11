// GPU version of `TurbulenceSolverCPU::apply_mirror_corrections`, in two phases:
//
// - `compute`: samples all transported fields at the mirrored image point of every entry, from the
//   uncorrected field, and stores the results in the compact `new_values` array.
// - `scatter`: writes `new_values` back into the field, in a separate dispatch.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<storage, read> entries: array<MirrorEntry>;
@group(0) @binding(2) var<storage, read_write> field: array<f32>;
@group(0) @binding(3) var<storage, read_write> new_values: array<f32>;

@compute @workgroup_size(WG_1D)
fn compute(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&entries) {
        return;
    }

    let base_index = entries[e].base_index;

    for (var f: u32 = 0u; f < NR_FIELDS; f = f + 1u) {
        let offset = f * N_EXTENDED_CELLS;

        var sum: f32 = 0.0;

        for (var a: u32 = 0u; a < 2u; a = a + 1u) {
            for (var b: u32 = 0u; b < 2u; b = b + 1u) {
                for (var c: u32 = 0u; c < 2u; c = c + 1u) {
                    let idx = base_index
                        + a * grid.extended_stride.x
                        + b * grid.extended_stride.y
                        + c * grid.extended_stride.z;

                    sum += entries[e].weights[a] * entries[e].weights[2u + b] * entries[e].weights[4u + c] * field[offset + idx];
                }
            }
        }

        new_values[e * NR_FIELDS + f] = sum;
    }
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

    let cell_index = entries[e].cell_index;

    for (var f: u32 = 0u; f < NR_FIELDS; f = f + 1u) {
        field[f * N_EXTENDED_CELLS + cell_index] = new_values[e * NR_FIELDS + f];
    }
}
