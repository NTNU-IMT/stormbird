// Data exchange with the actuator line model, which runs on the CPU, restricted to the cells in
// `cell_indices` (flat extended indices of interior cells):
//
// - `gather`: the cell-centered velocity of each cell, matching
//   `Grid::cell_centered_value_from_face_staggered`, written to `cell_values`.
// - `scatter`: writes `cell_values` (the body force of each cell) into `body_force`.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<storage, read> cell_indices: array<u32>;
@group(0) @binding(2) var<storage, read> velocity: array<f32>;
@group(0) @binding(3) var<storage, read_write> body_force: array<f32>;
@group(0) @binding(4) var<storage, read_write> cell_values: array<f32>;

@compute @workgroup_size(WG_1D)
fn gather(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&cell_indices) {
        return;
    }

    let i_0 = cell_indices[e];

    for (var c: u32 = 0u; c < 3u; c = c + 1u) {
        let i_n = i_0 - grid.extended_stride[c];

        cell_values[3u * e + c] = 0.5 * (velocity[3u * i_0 + c] + velocity[3u * i_n + c]);
    }
}

@compute @workgroup_size(WG_1D)
fn scatter(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&cell_indices) {
        return;
    }

    let i_0 = cell_indices[e];

    for (var c: u32 = 0u; c < 3u; c = c + 1u) {
        body_force[3u * i_0 + c] = cell_values[3u * e + c];
    }
}
