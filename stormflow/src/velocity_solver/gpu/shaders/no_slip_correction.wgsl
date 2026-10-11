// GPU version of `correct_no_slip_entry_kernel`, applied to all entries in `NoSlipCorrections` in a
// single dispatch. Each entry targets a unique component of a unique cell, and only reads the value
// it writes, so the entries are fully independent.

// Matches `GpuNoSlipEntry` on the Rust side.
struct NoSlipEntry {
    // Index into the flat vector field: 3 * cell_index + axis_index
    field_index: u32,
    mu: f32,
}

@group(0) @binding(0) var<storage, read> entries: array<NoSlipEntry>;
@group(0) @binding(1) var<storage, read_write> field: array<f32>;

@compute @workgroup_size(WG_1D)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&entries) {
        return;
    }

    let item = entries[e];

    field[item.field_index] = item.mu * field[item.field_index] + (1.0 - item.mu) * 1e-6;
}
