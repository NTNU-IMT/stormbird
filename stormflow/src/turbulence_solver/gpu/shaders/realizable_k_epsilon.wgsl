// GPU version of the realizable k-epsilon model. See `turbulence_solver::models::
// realizable_k_epsilon` for the details. The model coefficients (A0, C2, INV_SIGMA_K,
// INV_SIGMA_EPSILON, K_MIN and EPSILON_MIN) are generated from the Rust side, and
// model_bindings.wgsl and transport.wgsl are prepended.

const K: u32 = 0u;
const EPSILON: u32 = 1u;

const PRODUCTION: u32 = 0u;
const EPSILON_SOURCE: u32 = 1u;

const SMALL: f32 = 1e-30;

struct GradientInvariants {
    s2: f32,
    mag_s: f32,
    production_factor: f32,
    s_cubed: f32,
    omega2: f32,
}

fn gradient_invariants(i_0: u32) -> GradientInvariants {
    var g = velocity_gradient(i_0);

    let trace = g[0][0] + g[1][1] + g[2][2];

    var s: array<vec3<f32>, 3>;
    var s2: f32 = 0.0;
    var production_factor: f32 = 0.0;
    var omega2: f32 = 0.0;

    for (var i: u32 = 0u; i < 3u; i = i + 1u) {
        for (var j: u32 = 0u; j < 3u; j = j + 1u) {
            let two_symm = g[i][j] + g[j][i];

            var dev_two_symm = two_symm;

            if i == j {
                dev_two_symm = two_symm - (2.0 / 3.0) * trace;
            }

            s[i][j] = 0.5 * dev_two_symm;
            s2 += 2.0 * s[i][j] * s[i][j];

            production_factor += g[i][j] * dev_two_symm;

            let skew = 0.5 * (g[i][j] - g[j][i]);
            omega2 += skew * skew;
        }
    }

    var s_cubed: f32 = 0.0;

    for (var i: u32 = 0u; i < 3u; i = i + 1u) {
        for (var j: u32 = 0u; j < 3u; j = j + 1u) {
            for (var k: u32 = 0u; k < 3u; k = k + 1u) {
                s_cubed += s[i][j] * s[j][k] * s[k][i];
            }
        }
    }

    return GradientInvariants(s2, sqrt(s2), production_factor, s_cubed, omega2);
}

// See `RealizableKEpsilon::auxiliary_kernel`
@compute @workgroup_size(WG_X, WG_Y, WG_Z)
fn compute_auxiliary_fields(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i_0 = interior_cell_index(gid, grid.interior_shape.xyz, grid.extended_stride.xyz);

    if i_0 >= N_EXTENDED_CELLS {
        return;
    }

    let invariants = gradient_invariants(i_0);

    let k = fields_old[K * N_EXTENDED_CELLS + i_0];
    let epsilon = fields_old[EPSILON * N_EXTENDED_CELLS + i_0];

    let eta = invariants.mag_s * k / epsilon;
    let c1 = max(eta / (5.0 + eta), 0.43);

    auxiliary_fields[PRODUCTION * N_EXTENDED_CELLS + i_0] = eddy_viscosity[i_0] * invariants.production_factor;
    auxiliary_fields[EPSILON_SOURCE * N_EXTENDED_CELLS + i_0] = c1 * invariants.mag_s;
}

// See `RealizableKEpsilon::wall_function_kernel`
@compute @workgroup_size(WG_1D)
fn compute_wall_functions(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>
) {
    let e = linear_index(gid, nwg);

    if e >= arrayLength(&wall_entries) {
        return;
    }

    let cell_index = wall_entries[e].cell_index;

    let k = fields_old[K * N_EXTENDED_CELLS + cell_index];
    let y = wall_entries[e].wall_distance;

    let sqrt_k = sqrt(k);
    let y_plus = params.c_mu_25 * y * sqrt_k / params.viscosity;

    if y_plus > params.y_plus_lam {
        let mag_grad_u_wall = tangential_reference_velocity(e) / wall_entries[e].reference_distance;

        let nu_wall = params.viscosity * y_plus * params.kappa / log(params.e * y_plus);

        auxiliary_fields[PRODUCTION * N_EXTENDED_CELLS + cell_index] =
            nu_wall * mag_grad_u_wall * params.c_mu_25 * sqrt_k / (params.kappa * y);

        wall_values[e] = params.c_mu_75 * k * sqrt_k / (params.kappa * y);
    } else {
        auxiliary_fields[PRODUCTION * N_EXTENDED_CELLS + cell_index] = 0.0;

        wall_values[e] = 2.0 * k * params.viscosity / (y * y);
    }
}

// See `RealizableKEpsilon::transport_kernel`
@compute @workgroup_size(WG_X, WG_Y, WG_Z)
fn transport_iteration(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i_0 = interior_cell_index(gid, grid.interior_shape.xyz, grid.extended_stride.xyz);

    if i_0 >= N_EXTENDED_CELLS {
        return;
    }

    let k = fields[K * N_EXTENDED_CELLS + i_0];
    let epsilon = fields[EPSILON * N_EXTENDED_CELLS + i_0];

    let inv_time_step = 1.0 / params.time_step;

    // --- k ---
    let k_transport = convection_diffusion(i_0, K, INV_SIGMA_K);

    let k_diagonal = inv_time_step + k_transport.x + epsilon / k;
    let k_rhs = fields_old[K * N_EXTENDED_CELLS + i_0] * inv_time_step +
        auxiliary_fields[PRODUCTION * N_EXTENDED_CELLS + i_0] +
        k_transport.y;

    // --- epsilon ---
    let epsilon_transport = convection_diffusion(i_0, EPSILON, INV_SIGMA_EPSILON);

    let epsilon_diagonal = inv_time_step + epsilon_transport.x +
        C2 * epsilon / (k + sqrt(params.viscosity * epsilon));
    let epsilon_rhs = fields_old[EPSILON * N_EXTENDED_CELLS + i_0] * inv_time_step +
        auxiliary_fields[EPSILON_SOURCE * N_EXTENDED_CELLS + i_0] * epsilon +
        epsilon_transport.y;

    fields_next[K * N_EXTENDED_CELLS + i_0] = max(k_rhs / k_diagonal, K_MIN);
    fields_next[EPSILON * N_EXTENDED_CELLS + i_0] = max(epsilon_rhs / epsilon_diagonal, EPSILON_MIN);
}

// See `RealizableKEpsilon::eddy_viscosity_kernel`
@compute @workgroup_size(WG_X, WG_Y, WG_Z)
fn compute_eddy_viscosity(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i_0 = interior_cell_index(gid, grid.interior_shape.xyz, grid.extended_stride.xyz);

    if i_0 >= N_EXTENDED_CELLS {
        return;
    }

    let invariants = gradient_invariants(i_0);

    let k = fields[K * N_EXTENDED_CELLS + i_0];
    let epsilon = fields[EPSILON * N_EXTENDED_CELLS + i_0];

    let w = (2.0 * sqrt(2.0)) * invariants.s_cubed / (invariants.mag_s * invariants.s2 + SMALL);

    let phi_s = (1.0 / 3.0) * acos(clamp(sqrt(6.0) * w, -1.0, 1.0));
    let a_s = sqrt(6.0) * cos(phi_s);
    let u_s = sqrt(0.5 * invariants.s2 + invariants.omega2);

    let c_mu = 1.0 / (A0 + a_s * u_s * k / epsilon);

    eddy_viscosity[i_0] = min(c_mu * k * k / epsilon, params.max_eddy_viscosity);
}
