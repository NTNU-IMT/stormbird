// GPU version of `VelocitySolverCPU::max_velocity`: the largest magnitude of the (staggered)
// velocity vectors over all N_EXTENDED_CELLS cells. Each invocation first reduces a strided subset
// of the cells, then each workgroup reduces in shared memory, and finally one atomic max per
// workgroup combines the results. `result` must be cleared to zero before the dispatch.
//
// The magnitudes are non-negative, so their bit patterns, interpreted as u32, have the same
// ordering as the f32 values themselves, which is what allows an integer atomic max. NaN values are
// skipped, matching `Float::max` on the CPU side.

@group(0) @binding(0) var<storage, read> velocity: array<f32>;
@group(0) @binding(1) var<storage, read_write> result: atomic<u32>;

var<workgroup> partial_max: array<f32, WG_REDUCE>;

fn is_nan(value: f32) -> bool {
    return (bitcast<u32>(value) & 0x7fffffffu) > 0x7f800000u;
}

@compute @workgroup_size(WG_REDUCE)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_index) lid: u32,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let total_nr_invocations = nwg.x * WG_REDUCE;

    var local_max: f32 = 0.0;

    for (var i: u32 = gid.x; i < N_EXTENDED_CELLS; i = i + total_nr_invocations) {
        let x = velocity[3u * i];
        let y = velocity[3u * i + 1u];
        let z = velocity[3u * i + 2u];

        let magnitude = sqrt(x * x + y * y + z * z);

        if !is_nan(magnitude) {
            local_max = max(local_max, magnitude);
        }
    }

    partial_max[lid] = local_max;

    workgroupBarrier();

    for (var offset: u32 = WG_REDUCE / 2u; offset > 0u; offset = offset >> 1u) {
        if lid < offset {
            partial_max[lid] = max(partial_max[lid], partial_max[lid + offset]);
        }

        workgroupBarrier();
    }

    if lid == 0u {
        atomicMax(&result, bitcast<u32>(partial_max[0]));
    }
}
