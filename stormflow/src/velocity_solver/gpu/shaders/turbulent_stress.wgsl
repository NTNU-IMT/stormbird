// GPU version of `convect_and_diffuse_turbulent_kernel` and `turbulent_stress_terms`. See those
// functions for the details. Appended to convect_and_diffuse.wgsl, whose functions it uses.

@group(0) @binding(6) var<storage, read> eddy_viscosity: array<f32>;

@compute @workgroup_size(WG_X, WG_Y, WG_Z)
fn main_turbulent(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i_0 = interior_cell_index(gid);

    if i_0 >= N_EXTENDED_CELLS {
        return;
    }

    let rate = explicit_rate(i_0);

    let stride = grid.extended_stride.xyz;

    var neighbor_terms = vec3<f32>(0.0, 0.0, 0.0);
    var diagonal = vec3<f32>(0.0, 0.0, 0.0);

    for (var i: u32 = 0u; i < 3u; i = i + 1u) {
        let s_i = stride[i];

        // The two cells on each side of the u_i-face
        let c_0 = i_0;
        let c_1 = i_0 + s_i;

        let nu_c_0 = eddy_viscosity[c_0];
        let nu_c_1 = eddy_viscosity[c_1];

        // Normal stress, where the transpose doubles the term
        let inv_h2_i = grid.inv_cell_length_squared[i];

        diagonal[i] += 2.0 * (nu_c_1 + nu_c_0) * inv_h2_i;
        neighbor_terms[i] += 2.0 * (
            nu_c_1 * vel(i_0 + s_i, i) +
            nu_c_0 * vel(i_0 - s_i, i)
        ) * inv_h2_i;

        // Shear stresses, at the edges above and below the face along j
        for (var j: u32 = 0u; j < 3u; j = j + 1u) {
            if j == i {
                continue;
            }

            let s_j = stride[j];

            let nu_edge_p = 0.25 * (
                nu_c_0 + nu_c_1 + eddy_viscosity[c_0 + s_j] + eddy_viscosity[c_1 + s_j]
            );

            let nu_edge_m = 0.25 * (
                nu_c_0 + nu_c_1 + eddy_viscosity[c_0 - s_j] + eddy_viscosity[c_1 - s_j]
            );

            let inv_h2_j = grid.inv_cell_length_squared[j];

            diagonal[i] += (nu_edge_p + nu_edge_m) * inv_h2_j;
            neighbor_terms[i] += (
                nu_edge_p * vel(i_0 + s_j, i) +
                nu_edge_m * vel(i_0 - s_j, i)
            ) * inv_h2_j;

            let duj_dxi_p = (vel(i_0 + s_i, j) - vel(i_0, j)) * grid.inv_cell_length[i];
            let duj_dxi_m = (vel(i_0 + s_i - s_j, j) - vel(i_0 - s_j, j)) * grid.inv_cell_length[i];

            neighbor_terms[i] += (nu_edge_p * duj_dxi_p - nu_edge_m * duj_dxi_m) * grid.inv_cell_length[j];
        }
    }

    for (var c: u32 = 0u; c < 3u; c = c + 1u) {
        velocity_star[3u * i_0 + c] = (velocity_org[3u * i_0 + c] + params.time_step * (rate[c] + neighbor_terms[c])) /
            (1.0 + params.time_step * diagonal[c]);
    }
}
