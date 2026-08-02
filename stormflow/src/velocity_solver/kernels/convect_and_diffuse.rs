use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

#[inline(always)]
/// 4th order accurate interpolation of a 1D sequence of samples spaced by a constant `h` onto
/// the midpoint between `f_0` and `f_1`, given the two flanking samples `f_m1` (one `h` before
/// `f_0`) and `f_2` (one `h` after `f_1`).
fn interp4(f_m1: Float, f_0: Float, f_1: Float, f_2: Float) -> Float {
    (9.0 * (f_0 + f_1) - (f_m1 + f_2)) * (1.0 / 16.0)
}

#[inline(always)]
/// 4th order accurate upwind-biased first derivative at `x0`, for the case the transport
/// velocity points in the `+` direction (so the stencil leans 3 points upwind, 1 point
/// downwind): samples at `x0 - 3h, x0 - 2h, x0 - h, x0, x0 + h`. Caller still has to scale the
/// result by `1 / h`. Central differencing here is unconditionally unstable for advection under
/// this solver's explicit time stepping (central space + Euler-type time integration has no
/// stability region on the imaginary axis), so this upwind bias is what actually provides the
/// numerical dissipation needed for stability — same role the old 1st order upwind switch played.
fn upwind_derivative4_plus(f_m3: Float, f_m2: Float, f_m1: Float, f_0: Float, f_p1: Float) -> Float {
    (-f_m3 + 6.0 * f_m2 - 18.0 * f_m1 + 10.0 * f_0 + 3.0 * f_p1) * (1.0 / 12.0)
}

#[inline(always)]
/// Mirror image of [`upwind_derivative4_plus`], for the transport velocity pointing in the `-`
/// direction: samples at `x0 - h, x0, x0 + h, x0 + 2h, x0 + 3h`.
fn upwind_derivative4_minus(f_m1: Float, f_0: Float, f_p1: Float, f_p2: Float, f_p3: Float) -> Float {
    (-3.0 * f_m1 - 10.0 * f_0 + 18.0 * f_p1 - 6.0 * f_p2 + f_p3) * (1.0 / 12.0)
}

#[inline(always)]
/// 4th order accurate central second derivative at `x0`, given samples at `x0`, `x0 - 2h`,
/// `x0 - h`, `x0 + h` and `x0 + 2h`. Caller still has to scale the result by `1 / h^2`.
fn laplacian4(f_m2: Float, f_m1: Float, f_0: Float, f_p1: Float, f_p2: Float) -> Float {
    (16.0 * (f_m1 + f_p1) - (f_m2 + f_p2) - 30.0 * f_0) * (1.0 / 12.0)
}

#[inline(always)]
/// 4th order accurate interpolation of the `component` velocity field (stored on `component`-
/// faces, i.e. naturally sampled at cell centers along every other axis) onto the center of cell
/// `col` along `stride`'s axis — the midpoint between that cell's two bounding faces.
fn face_to_cell_center(velocity: &[SpatialVector], col: usize, stride: usize, component: usize) -> Float {
    interp4(
        velocity[col - 2 * stride][component],
        velocity[col - stride][component],
        velocity[col][component],
        velocity[col + stride][component],
    )
}

#[inline(always)]
pub fn convect_and_diffuse_kernel(
    i_0: usize,
    grid: &Grid,
    velocity_org: &[SpatialVector],
    velocity: &[SpatialVector],
    body_force: &[SpatialVector],
    viscosity: Float,
    inv_density: Float,
    time_step: Float
) -> SpatialVector {
    let i_p = [i_0 + grid.extended_stride[0], i_0 + grid.extended_stride[1], i_0 + grid.extended_stride[2]];
    let i_n = [i_0 - grid.extended_stride[0], i_0 - grid.extended_stride[1], i_0 - grid.extended_stride[2]];
    let i_p2 = [i_p[0] + grid.extended_stride[0], i_p[1] + grid.extended_stride[1], i_p[2] + grid.extended_stride[2]];
    let i_n2 = [i_n[0] - grid.extended_stride[0], i_n[1] - grid.extended_stride[1], i_n[2] - grid.extended_stride[2]];
    let i_p3 = [i_p2[0] + grid.extended_stride[0], i_p2[1] + grid.extended_stride[1], i_p2[2] + grid.extended_stride[2]];
    let i_n3 = [i_n2[0] - grid.extended_stride[0], i_n2[1] - grid.extended_stride[1], i_n2[2] - grid.extended_stride[2]];

    let mut new_value = SpatialVector::default();

    let v0 = velocity[i_0];

    // ---------------- Convective and diffusive terms --------------
    for vel_comp in 0..3 {
        let u_i = v0[vel_comp];

        for deriv_dir in 0..3 {
            // -------------- Convection ------------------------
            let u_j = if vel_comp == deriv_dir {
                // u_vel_comp interpolated onto its own location is exact, not an approximation.
                v0[deriv_dir]
            } else {
                // Tensor-product 4th order interpolation of the deriv_dir-velocity component
                // onto u_i's location: first interpolate deriv_dir-faces to cell centers (along
                // deriv_dir) at 4 columns spanning u_i's stencil in the vel_comp direction, then
                // interpolate those cell-center values onto u_i's face (along vel_comp).
                let stride_deriv = grid.extended_stride[deriv_dir];

                interp4(
                    face_to_cell_center(velocity, i_n[vel_comp], stride_deriv, deriv_dir),
                    face_to_cell_center(velocity, i_0, stride_deriv, deriv_dir),
                    face_to_cell_center(velocity, i_p[vel_comp], stride_deriv, deriv_dir),
                    face_to_cell_center(velocity, i_p2[vel_comp], stride_deriv, deriv_dir),
                )
            };

            let f_m2 = velocity[i_n2[deriv_dir]][vel_comp];
            let f_m1 = velocity[i_n[deriv_dir]][vel_comp];
            let f_p1 = velocity[i_p[deriv_dir]][vel_comp];
            let f_p2 = velocity[i_p2[deriv_dir]][vel_comp];

            let dui_dxj = if u_j > 0.0 {
                let f_m3 = velocity[i_n3[deriv_dir]][vel_comp];

                upwind_derivative4_plus(f_m3, f_m2, f_m1, u_i, f_p1) * grid.inv_cell_length[deriv_dir]
            } else {
                let f_p3 = velocity[i_p3[deriv_dir]][vel_comp];

                upwind_derivative4_minus(f_m1, u_i, f_p1, f_p2, f_p3) * grid.inv_cell_length[deriv_dir]
            };

            new_value[vel_comp] -= u_j * dui_dxj;

            // ------------- Diffusion ------------------------
            new_value[vel_comp] += viscosity *
                laplacian4(f_m2, f_m1, u_i, f_p1, f_p2) *
                grid.inv_cell_length_squared[deriv_dir];
        }

        // ------------- Body force ------------------------
        // 4th order accurate interpolation of body_force (cell-centered) onto the staggered
        // face between i_0 and i_p[vel_comp]. The ghost layers on every side guarantee this
        // 4-point stencil is available for every interior cell, so no boundary fallback is
        // needed.
        new_value[vel_comp] += interp4(
            body_force[i_n[vel_comp]][vel_comp],
            body_force[i_0][vel_comp],
            body_force[i_p[vel_comp]][vel_comp],
            body_force[i_p2[vel_comp]][vel_comp],
        ) * inv_density;
    }

    velocity_org[i_0] + time_step * new_value
}
