// Damping of the eddy viscosity inside the no-slip geometries, see `DampingEntry`

@group(0) @binding(0) var<storage, read> entries: array<DampingEntry>;
@group(0) @binding(1) var<storage, read_write> eddy_viscosity: array<f32>;

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

    eddy_viscosity[item.cell_index] = item.mu * eddy_viscosity[item.cell_index];
}
