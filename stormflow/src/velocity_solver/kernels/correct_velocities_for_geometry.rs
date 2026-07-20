
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::geometry::Geometry;

#[inline(always)]
pub fn correct_velocities_for_no_slip_geometry_kernel(
    i_0: usize,
    grid: &Grid,
    current_velocity: SpatialVector,
    signed_distance_function: &[Float],
    epsilon: Float
) -> SpatialVector {
    let extended_indices = grid.extended_indices_from_flat_index(i_0);

    let mut new_velocity = SpatialVector::default();
    
    for axis_index in 0..3 {
        let mut extended_indices_p = extended_indices;
        extended_indices_p[axis_index] += 1;

        let i_p = grid.flat_index_on_extended_grid(extended_indices_p);
        
        let sdf = 0.5 * (
            signed_distance_function[i_0] + 
            signed_distance_function[i_p]
        );
        
        let mu = Geometry::blending_function(sdf, epsilon);
        
        new_velocity[axis_index] = mu * current_velocity[axis_index] + (1.0 - mu) * 1e-6;
    }

    new_velocity
}

#[inline(always)]
pub fn correct_velocities_for_slip_geometry_kernel(
    i_0: usize,
    grid: &Grid,
    current_velocity: SpatialVector,
    signed_distance_function: &[Float],
    normals: &[SpatialVector],
    epsilon: Float
) -> SpatialVector {
    let extended_indices = grid.extended_indices_from_flat_index(i_0);

    let mut new_velocity = SpatialVector::default();
    
    for axis_index in 0..3 {
        let mut extended_indices_p = extended_indices;
        extended_indices_p[axis_index] += 1;

        let i_p = grid.flat_index_on_extended_grid(extended_indices_p);
        
        let sdf = 0.5 * (
            signed_distance_function[i_0] + 
            signed_distance_function[i_p]
        );

        let normal = 0.5 * (
            normals[i_0] + 
            normals[i_p]
        );
        
        let mu = Geometry::blending_function(sdf, epsilon);

        let normal_component = normal[axis_index];

        let vel_normal_contribution = current_velocity[axis_index] * normal_component * normal_component;

        new_velocity[axis_index] = current_velocity[axis_index] - (1.0 - mu) * vel_normal_contribution;
    }

    new_velocity
}