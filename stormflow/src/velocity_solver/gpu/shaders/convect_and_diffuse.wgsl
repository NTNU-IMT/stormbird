// GPU version of `convect_and_diffuse_kernel`. See that function for the details of the numerics.
// Dispatched over the interior cells, with gid.x along the contiguous z-axis.

@group(0) @binding(0) var<uniform> grid: Grid;
@group(0) @binding(1) var<uniform> params: VelocityParams;
@group(0) @binding(2) var<storage, read> velocity_org: array<f32>;
@group(0) @binding(3) var<storage, read> velocity: array<f32>;
@group(0) @binding(4) var<storage, read> body_force: array<f32>;
@group(0) @binding(5) var<storage, read_write> velocity_star: array<f32>;

fn vel(i: u32, component: u32) -> f32 {
    return velocity[3u * i + component];
}

fn force(i: u32, component: u32) -> f32 {
    return body_force[3u * i + component];
}

fn upwind_derivative4_plus(f_m3: f32, f_m2: f32, f_m1: f32, f_0: f32, f_p1: f32) -> f32 {
    return (-f_m3 + 6.0 * f_m2 - 18.0 * f_m1 + 10.0 * f_0 + 3.0 * f_p1) * (1.0 / 12.0);
}

fn upwind_derivative4_minus(f_m1: f32, f_0: f32, f_p1: f32, f_p2: f32, f_p3: f32) -> f32 {
    return (-3.0 * f_m1 - 10.0 * f_0 + 18.0 * f_p1 - 6.0 * f_p2 + f_p3) * (1.0 / 12.0);
}

fn laplacian4(f_m2: f32, f_m1: f32, f_0: f32, f_p1: f32, f_p2: f32) -> f32 {
    return (16.0 * (f_m1 + f_p1) - (f_m2 + f_p2) - 30.0 * f_0) * (1.0 / 12.0);
}

fn face_to_cell_center(col: u32, stride: u32, component: u32) -> f32 {
    return interp4(
        vel(col - 2u * stride, component),
        vel(col - stride, component),
        vel(col, component),
        vel(col + stride, component),
    );
}

@compute @workgroup_size(WG_X, WG_Y, WG_Z)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ki = gid.x;
    let ji = gid.y;
    let ii = gid.z;

    if ii >= grid.interior_shape.x || ji >= grid.interior_shape.y || ki >= grid.interior_shape.z {
        return;
    }

    let stride = grid.extended_stride.xyz;

    let i_0 = (ii + INTERIOR_OFFSET) * stride.x + (ji + INTERIOR_OFFSET) * stride.y + (ki + INTERIOR_OFFSET);

    var new_value = array<f32, 3>(0.0, 0.0, 0.0);

    for (var vel_comp: u32 = 0u; vel_comp < 3u; vel_comp = vel_comp + 1u) {
        let u_i = vel(i_0, vel_comp);
        let stride_vel = stride[vel_comp];

        for (var deriv_dir: u32 = 0u; deriv_dir < 3u; deriv_dir = deriv_dir + 1u) {
            let stride_deriv = stride[deriv_dir];

            // -------------- Convection ------------------------
            var u_j: f32;

            if vel_comp == deriv_dir {
                u_j = vel(i_0, deriv_dir);
            } else {
                u_j = interp4(
                    face_to_cell_center(i_0 - stride_vel, stride_deriv, deriv_dir),
                    face_to_cell_center(i_0, stride_deriv, deriv_dir),
                    face_to_cell_center(i_0 + stride_vel, stride_deriv, deriv_dir),
                    face_to_cell_center(i_0 + 2u * stride_vel, stride_deriv, deriv_dir),
                );
            }

            let f_m2 = vel(i_0 - 2u * stride_deriv, vel_comp);
            let f_m1 = vel(i_0 - stride_deriv, vel_comp);
            let f_p1 = vel(i_0 + stride_deriv, vel_comp);
            let f_p2 = vel(i_0 + 2u * stride_deriv, vel_comp);

            var dui_dxj: f32;

            if u_j > 0.0 {
                let f_m3 = vel(i_0 - 3u * stride_deriv, vel_comp);

                dui_dxj = upwind_derivative4_plus(f_m3, f_m2, f_m1, u_i, f_p1) * grid.inv_cell_length[deriv_dir];
            } else {
                let f_p3 = vel(i_0 + 3u * stride_deriv, vel_comp);

                dui_dxj = upwind_derivative4_minus(f_m1, u_i, f_p1, f_p2, f_p3) * grid.inv_cell_length[deriv_dir];
            }

            new_value[vel_comp] -= u_j * dui_dxj;

            // ------------- Diffusion ------------------------
            new_value[vel_comp] += params.viscosity *
                laplacian4(f_m2, f_m1, u_i, f_p1, f_p2) *
                grid.inv_cell_length_squared[deriv_dir];
        }

        // ------------- Body force ------------------------
        new_value[vel_comp] += interp4(
            force(i_0 - stride_vel, vel_comp),
            force(i_0, vel_comp),
            force(i_0 + stride_vel, vel_comp),
            force(i_0 + 2u * stride_vel, vel_comp),
        ) * params.inv_density;
    }

    for (var c: u32 = 0u; c < 3u; c = c + 1u) {
        velocity_star[3u * i_0 + c] = velocity_org[3u * i_0 + c] + params.time_step * new_value[c];
    }
}
