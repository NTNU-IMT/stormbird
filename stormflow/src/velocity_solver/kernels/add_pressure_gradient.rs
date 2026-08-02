use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

#[inline(always)]
pub fn add_pressure_gradient_kernel(
    i_0: usize,
    grid: &Grid,
    pressure: &[Float],
    velocity_star: &[SpatialVector],
    inv_density: Float,
    time_step: Float
) -> SpatialVector {
    let mut dp_dx = SpatialVector::default();

    // 4th order accurate pressure gradient onto u_i's face: the same symmetric 4-point
    // staggered derivative as `PressureSolver::calculate_rhs`'s divergence, in the opposite
    // staggering direction (cell-centered pressure -> face gradient instead of face-centered
    // velocity -> cell-center divergence).
    for axis_index in 0..3 {
        let stride = grid.extended_stride[axis_index];

        let i_n = i_0 - stride;
        let i_p = i_0 + stride;
        let i_p2 = i_p + stride;

        dp_dx[axis_index] = (
            27.0 * (pressure[i_p] - pressure[i_0]) -
            (pressure[i_p2] - pressure[i_n])
        ) * grid.inv_cell_length[axis_index] * (1.0 / 24.0);
    }

    velocity_star[i_0] - time_step * inv_density * dp_dx
}