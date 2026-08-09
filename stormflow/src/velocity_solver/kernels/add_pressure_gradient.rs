use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;

#[inline(always)]
pub fn add_pressure_gradient_kernel(
    i_0: usize,
    extended_indices: [usize; 3],
    grid: &Grid,
    pressure: &[Float],
    velocity_star: &[SpatialVector],
    inv_density: Float,
    time_step: Float
) -> SpatialVector {
    let mut dp_dx = SpatialVector::default();

    // 4th order accurate pressure gradient onto u_i's face: the staggered 4-point derivative that
    // takes cell-centered samples to the positive face. It is the exact counterpart of
    // `PressureSolver::calculate_rhs`'s divergence, which takes face samples back to the cell
    // center. On a non-uniform grid the two pressures straddling the face are not `dx` apart, so
    // the weights come from the grid's stencil tables rather than from a single cell length; on a
    // uniform grid they reduce to the familiar `(27 * (p_p - p_0) - (p_p2 - p_n)) / (24 dx)`.
    for axis_index in 0..3 {
        let stride = grid.extended_stride[axis_index];

        let i_n = i_0 - stride;
        let i_p = i_0 + stride;
        let i_p2 = i_p + stride;

        let weights = &grid.stencils[axis_index].gradient_center_to_face[extended_indices[axis_index]];

        dp_dx[axis_index] =
            weights[0] * pressure[i_n] +
            weights[1] * pressure[i_0] +
            weights[2] * pressure[i_p] +
            weights[3] * pressure[i_p2];
    }

    velocity_star[i_0] - time_step * inv_density * dp_dx
}
