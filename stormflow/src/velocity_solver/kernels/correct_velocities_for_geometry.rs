
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::geometry::Geometry;
use crate::velocity_solver::slip_mirror_stencils::SlipMirrorEntry;

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
/// Free-slip correction via a mirror (ghost-cell) construction, for one precomputed `entry`
/// (one staggered face, i.e. one cell/axis pair): the mirrored image point's velocity is
/// reconstructed by tricubic interpolation (one stencil per component, since each component is
/// staggered on a different face), reflected across the local normal (`v - 2 (v . n) n`), and
/// blended in with `entry.mu`. All of the interface-mirroring geometry (which cell, the image
/// point, the interpolation stencils, the reflection normal, `mu`) is precomputed once in
/// `SlipMirrorStencils::build`, since the slip geometry is static — this kernel is a pure
/// gather-and-blend.
///
/// `axis_index` selects which of `entry`'s three component stencils' *targets* is being written;
/// reconstructing `v_image` still needs all three components regardless, since the reflection
/// `v - 2 (v . n) n` depends on the full vector.
pub fn correct_slip_mirror_entry_kernel(
    entry: &SlipMirrorEntry,
    velocity_snapshot: &[SpatialVector],
    stride: [usize; 3],
    axis_index: usize,
) -> Float {
    let mut v_image = SpatialVector::default();

    for component in 0..3 {
        v_image[component] = entry.component_stencils[component]
            .sample_component(velocity_snapshot, component, stride);
    }

    let v_image_dot_normal = v_image.dot(entry.normal);
    let ghost_velocity = v_image[axis_index] - 2.0 * v_image_dot_normal * entry.normal[axis_index];

    let current = velocity_snapshot[entry.cell_index][axis_index];

    entry.mu * current + (1.0 - entry.mu) * ghost_velocity
}