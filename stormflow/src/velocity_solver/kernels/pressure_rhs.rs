use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

#[inline(always)]
/// Right hand side of the pressure Poisson equation for one interior cell: the divergence of
/// `velocity_star`, scaled by `density / time_step`.
pub fn pressure_rhs_kernel(
    i_flat_interior: usize,
    grid: &Grid,
    velocity_star: &[SpatialVector],
    density: Float,
    inv_time_step: Float
) -> Float {
    let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
    let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
    let i_0 = grid.flat_index_on_extended_grid(extended_indices);

    let mut divergence = 0.0;

    // 4th order accurate divergence: the standard symmetric 4-point staggered derivative (see
    // `add_pressure_gradient_kernel`, which uses the same formula in the opposite staggering
    // direction).
    for axis_index in 0..3 {
        let stride = grid.extended_stride[axis_index];

        let i_n = i_0 - stride;
        let i_p = i_0 + stride;
        let i_n2 = i_n - stride;

        divergence += (
            27.0 * (velocity_star[i_0][axis_index] - velocity_star[i_n][axis_index]) -
            (velocity_star[i_p][axis_index] - velocity_star[i_n2][axis_index])
        ) * grid.inv_cell_length[axis_index] * (1.0 / 24.0);
    }

    divergence * (density * inv_time_step)
}
