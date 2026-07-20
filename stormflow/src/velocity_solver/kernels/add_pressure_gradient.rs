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

    let extended_indices = grid.extended_indices_from_flat_index(i_0);

    for axis_index in 0..3 {
        let mut extended_indices_p = extended_indices;
        extended_indices_p[axis_index] += 1;

        let i_p = grid.flat_index_on_extended_grid(extended_indices_p);

        dp_dx[axis_index] = (
            pressure[i_p] - 
            pressure[i_0]
        ) * grid.inv_cell_length[axis_index];
    }
    
    let new_velocity = velocity_star[i_0] - time_step * inv_density * dp_dx;

    new_velocity
}