
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::velocity_solver::slip_mirror_stencils::SlipMirrorEntry;
use crate::velocity_solver::no_slip_corrections::NoSlipEntry;

#[inline(always)]
/// No-slip correction for one precomputed `entry` (one staggered face, i.e. one cell/axis pair):
/// blends the `current` velocity component of the face towards zero with `entry.mu`. The blend 
/// factor is precomputed once in `NoSlipCorrections::build`, since the geometry is static.
pub fn correct_no_slip_entry_kernel(
    entry: &NoSlipEntry,
    current: Float,
) -> Float {
    entry.mu * current + (1.0 - entry.mu) * 1e-6
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
/// `velocity` is only read, never written, so all entries sample the same uncorrected field. The
/// caller is responsible for not writing the results back until every entry has been computed.
///
/// `axis_index` selects which of `entry`'s three component stencils' *targets* is being written;
/// reconstructing `v_image` still needs all three components regardless, since the reflection
/// `v - 2 (v . n) n` depends on the full vector.
pub fn correct_slip_mirror_entry_kernel(
    entry: &SlipMirrorEntry,
    velocity: &[SpatialVector],
    stride: [usize; 3],
    axis_index: usize,
) -> Float {
    let mut v_image = SpatialVector::default();

    for component in 0..3 {
        v_image[component] = entry.component_stencils[component]
            .sample_component(velocity, component, stride);
    }

    let v_image_dot_normal = v_image.dot(entry.normal);
    let ghost_velocity = v_image[axis_index] - 2.0 * v_image_dot_normal * entry.normal[axis_index];

    let current = velocity[entry.cell_index][axis_index];

    entry.mu * current + (1.0 - entry.mu) * ghost_velocity
}