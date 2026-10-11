// GPU version of `TurbulenceSolverCPU::apply_wall_function_values`: fixes the wall function field
// (WALL_FUNCTION_FIELD) to the values computed by the model's `wall_functions` kernel.

@group(0) @binding(0) var<storage, read> wall_entries: array<WallFunctionEntry>;
@group(0) @binding(1) var<storage, read> wall_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> field: array<f32>;

@compute @workgroup_size(WG_1D)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&wall_entries) {
        return;
    }

    field[WALL_FUNCTION_FIELD * N_EXTENDED_CELLS + wall_entries[e].cell_index] = wall_values[e];
}
