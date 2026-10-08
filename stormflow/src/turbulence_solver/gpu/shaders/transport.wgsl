// GPU versions of the model independent functions in `turbulence_solver::transport`. See those
// functions for the details. Uses the bindings in model_bindings.wgsl.

const LIMITED_LINEAR: u32 = 1u;

fn vel(i: u32, component: u32) -> f32 {
    return velocity[3u * i + component];
}

// The velocity gradient tensor at the center of cell `i_0`, where `g[i][j] = du_i/dx_j`. See
// `velocity_gradient`.
fn velocity_gradient(i_0: u32) -> array<vec3<f32>, 3> {
    let stride = grid.extended_stride.xyz;

    var g: array<vec3<f32>, 3>;

    for (var i: u32 = 0u; i < 3u; i = i + 1u) {
        let s_i = stride[i];

        for (var j: u32 = 0u; j < 3u; j = j + 1u) {
            if i == j {
                g[i][j] = (vel(i_0, i) - vel(i_0 - s_i, i)) * grid.inv_cell_length[i];
            } else {
                let s_j = stride[j];

                let u_p = vel(i_0 + s_j, i) + vel(i_0 + s_j - s_i, i);
                let u_m = vel(i_0 - s_j, i) + vel(i_0 - s_j - s_i, i);

                g[i][j] = 0.25 * (u_p - u_m) * grid.inv_cell_length[j];
            }
        }
    }

    return g;
}

fn limited_linear_correction(f_uu: f32, f_u: f32, f_d: f32) -> f32 {
    let delta = f_d - f_u;

    if abs(delta) < 1e-30 {
        return 0.0;
    }

    let r = (f_u - f_uu) / delta;

    return 0.5 * clamp(2.0 * r, 0.0, 1.0) * delta;
}

// The convection and diffusion of transported field `field_index` in cell `i_0`, as
// `vec2(diagonal, neighbors)`. See `convection_diffusion`.
fn convection_diffusion(i_0: u32, field_index: u32, inv_sigma: f32) -> vec2<f32> {
    let offset = field_index * N_EXTENDED_CELLS;
    let stride = grid.extended_stride.xyz;

    var diagonal: f32 = 0.0;
    var neighbors: f32 = 0.0;

    let f_0 = fields[offset + i_0];
    let nu_t_0 = eddy_viscosity[i_0];

    for (var axis: u32 = 0u; axis < 3u; axis = axis + 1u) {
        let s = stride[axis];
        let inv_h = grid.inv_cell_length[axis];
        let inv_h2 = grid.inv_cell_length_squared[axis];

        for (var side: u32 = 0u; side < 2u; side = side + 1u) {
            var outward_flux: f32;
            var i_n: u32;
            var i_nn: u32;
            var i_opposite: u32;

            if side == 0u {
                outward_flux = vel(i_0, axis);
                i_n = i_0 + s;
                i_nn = i_0 + 2u * s;
                i_opposite = i_0 - s;
            } else {
                outward_flux = -vel(i_0 - s, axis);
                i_n = i_0 - s;
                i_nn = i_0 - 2u * s;
                i_opposite = i_0 + s;
            }

            let f_n = fields[offset + i_n];

            let inflow = max(-outward_flux, 0.0) * inv_h;

            let diffusivity = params.viscosity + 0.5 * (nu_t_0 + eddy_viscosity[i_n]) * inv_sigma;
            let diffusion = diffusivity * inv_h2;

            diagonal += inflow + diffusion;
            neighbors += (inflow + diffusion) * f_n;

            if CONVECTION_SCHEME == LIMITED_LINEAR {
                var correction: f32;

                if outward_flux >= 0.0 {
                    correction = limited_linear_correction(fields[offset + i_opposite], f_0, f_n);
                } else {
                    correction = limited_linear_correction(fields[offset + i_nn], f_n, f_0);
                }

                neighbors -= outward_flux * correction * inv_h;
            }
        }
    }

    return vec2<f32>(diagonal, neighbors);
}

// Trilinear sample of velocity component `component` at the reference point of wall function
// entry `e`
fn sample_reference_velocity(e: u32, component: u32, base: u32) -> f32 {
    let w_offset = 6u * component;

    var sum: f32 = 0.0;

    for (var a: u32 = 0u; a < 2u; a = a + 1u) {
        for (var b: u32 = 0u; b < 2u; b = b + 1u) {
            for (var c: u32 = 0u; c < 2u; c = c + 1u) {
                let idx = base
                    + a * grid.extended_stride.x
                    + b * grid.extended_stride.y
                    + c * grid.extended_stride.z;

                sum += wall_entries[e].weights[w_offset + a] *
                    wall_entries[e].weights[w_offset + 2u + b] *
                    wall_entries[e].weights[w_offset + 4u + c] *
                    vel(idx, component);
            }
        }
    }

    return sum;
}

// The magnitude of the velocity tangential to the wall at the reference point of wall function
// entry `e`. See `WallFunctionEntry::tangential_velocity`.
fn tangential_reference_velocity(e: u32) -> f32 {
    let v = vec3<f32>(
        sample_reference_velocity(e, 0u, wall_entries[e].base_index_u),
        sample_reference_velocity(e, 1u, wall_entries[e].base_index_v),
        sample_reference_velocity(e, 2u, wall_entries[e].base_index_w),
    );

    let normal = vec3<f32>(wall_entries[e].normal_x, wall_entries[e].normal_y, wall_entries[e].normal_z);

    let normal_velocity = dot(v, normal);

    return length(v - normal_velocity * normal);
}
