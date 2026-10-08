// GPU version of `limit_velocity` in the CPU version of the velocity solver: clips each component of
// `field` to the range [-VELOCITY_LIMIT, VELOCITY_LIMIT], and counts the clipped components in
// `counter`, which is only reset at the start of each time step. VELOCITY_LIMIT is generated from
// the Rust side. Dispatched over all 3 * N_EXTENDED_CELLS components with `dispatch_1d`.

@group(0) @binding(0) var<storage, read_write> field: array<f32>;
@group(0) @binding(1) var<storage, read_write> counter: atomic<u32>;

@compute @workgroup_size(WG_1D)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let i = linear_index(gid, nwg);

    if i >= 3u * N_EXTENDED_CELLS {
        return;
    }

    let value = field[i];

    if value > VELOCITY_LIMIT {
        field[i] = VELOCITY_LIMIT;
        atomicAdd(&counter, 1u);
    } else if value < -VELOCITY_LIMIT {
        field[i] = -VELOCITY_LIMIT;
        atomicAdd(&counter, 1u);
    }
}
