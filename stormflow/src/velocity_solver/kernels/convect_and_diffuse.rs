use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

#[inline(always)]
/// Contracts a 4-node stencil with the four samples it applies to.
fn apply4(weights: &[Float; 4], f: [Float; 4]) -> Float {
    weights[0] * f[0] + weights[1] * f[1] + weights[2] * f[2] + weights[3] * f[3]
}

#[inline(always)]
/// Contracts a 5-node stencil with the five samples it applies to.
fn apply5(weights: &[Float; 5], f: [Float; 5]) -> Float {
    weights[0] * f[0] + weights[1] * f[1] + weights[2] * f[2] + weights[3] * f[3] + weights[4] * f[4]
}

#[inline(always)]
/// 4th order accurate interpolation of the `component` velocity field (stored on `component`-
/// faces, i.e. naturally sampled at cell centers along every other axis) onto the center of cell
/// `col` along `stride`'s axis — the midpoint between that cell's two bounding faces.
///
/// `weights` comes from `AxisStencils::interpolate_face_to_center` for that axis, so the stencil
/// accounts for the local cell lengths instead of assuming they are all equal. All four columns
/// this is called for share the same index along `stride`'s axis, hence the same weights.
fn face_to_cell_center(
    velocity: &[SpatialVector],
    weights: &[Float; 4],
    col: usize,
    stride: usize,
    component: usize
) -> Float {
    apply4(
        weights,
        [
            velocity[col - 2 * stride][component],
            velocity[col - stride][component],
            velocity[col][component],
            velocity[col + stride][component],
        ]
    )
}

#[inline(always)]
pub fn convect_and_diffuse_kernel(
    i_0: usize,
    extended_indices: [usize; 3],
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

        // `u_vel_comp` sits on cell `i_0`'s positive face along `vel_comp`. Along that axis its
        // samples are therefore face-staggered, while along the other two axes they sit at cell
        // centers — which is why every stencil below is picked from the "face" family when
        // `deriv_dir == vel_comp` and from the "center" family otherwise.
        let stencils_vel_comp = &grid.stencils[vel_comp];
        let index_vel_comp = extended_indices[vel_comp];

        for deriv_dir in 0..3 {
            let staggered = deriv_dir == vel_comp;

            let stencils_deriv = &grid.stencils[deriv_dir];
            let index_deriv = extended_indices[deriv_dir];

            // -------------- Convection ------------------------
            let u_j = if staggered {
                // u_vel_comp interpolated onto its own location is exact, not an approximation.
                v0[deriv_dir]
            } else {
                // Tensor-product 4th order interpolation of the deriv_dir-velocity component
                // onto u_i's location: first interpolate deriv_dir-faces to cell centers (along
                // deriv_dir) at 4 columns spanning u_i's stencil in the vel_comp direction, then
                // interpolate those cell-center values onto u_i's face (along vel_comp).
                let stride_deriv = grid.extended_stride[deriv_dir];
                let to_center = &stencils_deriv.interpolate_face_to_center[index_deriv];
                let to_face = &stencils_vel_comp.interpolate_center_to_face[index_vel_comp];

                apply4(
                    to_face,
                    [
                        face_to_cell_center(velocity, to_center, i_n[vel_comp], stride_deriv, deriv_dir),
                        face_to_cell_center(velocity, to_center, i_0, stride_deriv, deriv_dir),
                        face_to_cell_center(velocity, to_center, i_p[vel_comp], stride_deriv, deriv_dir),
                        face_to_cell_center(velocity, to_center, i_p2[vel_comp], stride_deriv, deriv_dir),
                    ]
                )
            };

            let f_m2 = velocity[i_n2[deriv_dir]][vel_comp];
            let f_m1 = velocity[i_n[deriv_dir]][vel_comp];
            let f_p1 = velocity[i_p[deriv_dir]][vel_comp];
            let f_p2 = velocity[i_p2[deriv_dir]][vel_comp];

            // The upwind bias is what provides the numerical dissipation this solver's explicit
            // time stepping needs; see the stencil tables for the coefficients, which reduce to
            // the classical `[-1, 6, -18, 10, 3]/(12h)` on a uniform grid.
            let dui_dxj = if u_j > 0.0 {
                let f_m3 = velocity[i_n3[deriv_dir]][vel_comp];

                let weights = if staggered {
                    &stencils_deriv.upwind_plus_face[index_deriv]
                } else {
                    &stencils_deriv.upwind_plus_center[index_deriv]
                };

                apply5(weights, [f_m3, f_m2, f_m1, u_i, f_p1])
            } else {
                let f_p3 = velocity[i_p3[deriv_dir]][vel_comp];

                let weights = if staggered {
                    &stencils_deriv.upwind_minus_face[index_deriv]
                } else {
                    &stencils_deriv.upwind_minus_center[index_deriv]
                };

                apply5(weights, [f_m1, u_i, f_p1, f_p2, f_p3])
            };

            new_value[vel_comp] -= u_j * dui_dxj;

            // ------------- Diffusion ------------------------
            let second_derivative_weights = if staggered {
                &stencils_deriv.second_derivative_face[index_deriv]
            } else {
                &stencils_deriv.second_derivative_center[index_deriv]
            };

            new_value[vel_comp] += viscosity * apply5(
                second_derivative_weights,
                [f_m2, f_m1, u_i, f_p1, f_p2]
            );
        }

        // ------------- Body force ------------------------
        // 4th order accurate interpolation of body_force (cell-centered) onto the staggered
        // face between i_0 and i_p[vel_comp]. The ghost layers on every side guarantee this
        // 4-point stencil is available for every interior cell, so no boundary fallback is
        // needed.
        new_value[vel_comp] += apply4(
            &stencils_vel_comp.interpolate_center_to_face[index_vel_comp],
            [
                body_force[i_n[vel_comp]][vel_comp],
                body_force[i_0][vel_comp],
                body_force[i_p[vel_comp]][vel_comp],
                body_force[i_p2[vel_comp]][vel_comp],
            ]
        ) * inv_density;
    }

    velocity_org[i_0] + time_step * new_value
}
