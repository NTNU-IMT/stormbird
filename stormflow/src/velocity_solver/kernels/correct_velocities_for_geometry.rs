
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
/// Free-slip correction via a mirror (ghost-cell) construction: for each staggered velocity
/// component, the interface is mirrored locally (assuming a unit-gradient signed distance
/// function) to find the image point on the fluid side, the velocity there is sampled by
/// trilinear interpolation, and that sample is reflected across the local normal
/// (`v - 2 (v . n) n`) before being blended in with `mu`.
///
/// Unlike `correct_velocities_for_slip_geometry_kernel`, which only ever looks at the current
/// cell's own velocity, this also constrains the tangential component near the wall instead of
/// leaving it free inside the body.
pub fn correct_velocities_for_slip_geometry_mirror_kernel(
    i_0: usize,
    grid: &Grid,
    current_velocity: SpatialVector,
    velocity_snapshot: &[SpatialVector],
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

        let normal = (
            0.5 * (normals[i_0] + normals[i_p])
        ).normalize();

        // One-sided: cells already in the fluid (sdf >= 0) must stay completely untouched.
        // Using the symmetric `blending_function` here would mirror *fluid* cells near the wall
        // across into the body and blend that spurious reflection into their own velocity,
        // corrupting the near-wall flow instead of just filling ghost values inside the body.
        let mu = if sdf >= 0.0 {
            1.0
        } else {
            Geometry::blending_function(sdf, epsilon)
        };

        if mu < 1.0 {
            let mut face_center = grid.cell_center_extended(extended_indices);
            face_center[axis_index] += 0.5 * grid.cell_length[axis_index];
    
            // Reflect the face location across the (locally linear) interface to get the image
            // point on the fluid side: `sdf` is negative inside the body, so this moves outward.
            let image_point = face_center - 2.0 * sdf * normal;
    
            let v_image = grid.interpolate_velocity(velocity_snapshot, image_point);
            let v_image_dot_normal = v_image.dot(normal);
    
            let ghost_velocity = v_image[axis_index] - 2.0 * v_image_dot_normal * normal[axis_index];
    
            new_velocity[axis_index] = mu * current_velocity[axis_index] + (1.0 - mu) * ghost_velocity;
        } else {
            new_velocity[axis_index] = current_velocity[axis_index];
        }
    }

    new_velocity
}