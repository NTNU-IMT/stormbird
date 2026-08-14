use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::grid::interpolation::{TrilinearStencil, TricubicStencil};
use crate::geometry::Geometry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// Which interpolation order is used to sample the mirrored image point for the slip-wall
/// *velocity* correction specifically — independent of
/// `pressure_solver::multigrid_cpu::slip_pressure_stencils::SlipPressureInterpolationOrder`, which
/// is the same choice for the pressure side. The two are deliberately separate settings (velocity
/// and pressure corrections live in unrelated solvers with no shared configuration today), but set
/// them to the same order if you want matching accuracy at the slip wall for both fields.
pub enum SlipMirrorInterpolationOrder {
    /// 2nd order accurate. Trilinear weights are always non-negative and sum to 1 (a true convex
    /// combination, unlike tricubic's), so this is the safer choice near thin/close-together
    /// walls, where the tricubic stencil's wider reach is more likely to pull in image points
    /// from the "wrong side" of a nearby second surface.
    Trilinear,
    /// 4th order accurate, matching the rest of the solver's accuracy order. Default.
    #[default]
    Tricubic,
}

#[derive(Debug, Clone, Copy)]
/// The interpolation stencil for one velocity component of one `SlipMirrorEntry`'s mirrored image
/// point, in whichever order `SlipMirrorInterpolationOrder` selected when the entries were built.
pub enum SlipMirrorInterpolationStencil {
    Trilinear(TrilinearStencil),
    Tricubic(TricubicStencil),
}

impl SlipMirrorInterpolationStencil {
    #[inline(always)]
    pub fn sample_component(&self, field: &[SpatialVector], component: usize, stride: [usize; 3]) -> Float {
        match self {
            Self::Trilinear(stencil) => stencil.sample_component(field, component, stride),
            Self::Tricubic(stencil) => stencil.sample_component(field, component, stride),
        }
    }
}

/// How many grid cells deep into a slip body the mirror correction is still computed, expressed
/// as a multiple of the largest cell length. Chosen to match `convect_and_diffuse`'s widest
/// stencil (the 4th order upwind derivative reaches 3 cells upstream): a body cell any deeper
/// than this can never be read by a real fluid cell's finite-difference/interpolation stencil, so
/// correcting it would have no effect on the simulation's output at any timestep.
pub const SLIP_MIRROR_REACH_CELLS: Float = 3.0;

#[derive(Debug, Clone)]
/// A precomputed slip-mirror correction for one staggered velocity face (one cell, one axis).
/// Everything geometry-dependent (which cell, how much to blend, the reflection normal, and the
/// interpolation stencils for sampling the mirrored image point) is computed once, since the
/// slip geometry is static; only the actual velocity values are re-sampled every step.
pub struct SlipMirrorEntry {
    /// Flat extended-grid index of the corrected face's base cell.
    pub cell_index: usize,
    /// Blend factor between the cell's own (post no-slip-correction) velocity and the mirrored
    /// ghost velocity: `0` deep inside the body, `1` in the fluid (such cells get no entry at all).
    pub mu: Float,
    pub normal: SpatialVector,
    /// One interpolation stencil per velocity field component (`u`, `v`, `w`), needed to
    /// reconstruct the full velocity vector at the mirrored image point since each component is
    /// staggered on a different face and therefore has its own stencil.
    pub component_stencils: [SlipMirrorInterpolationStencil; 3],
}

#[derive(Debug, Clone, Default)]
pub struct SlipMirrorStencils {
    /// One entry list per corrected velocity component/axis.
    pub entries: [Vec<SlipMirrorEntry>; 3],
}

impl SlipMirrorStencils {
    /// Builds the slip-mirror correction entries for every interior cell/axis within
    /// `reach_distance` of a slip surface, sampling the mirrored image point in the given
    /// interpolation `order`. Mirrors the per-cell geometry math that
    /// `correct_velocities_for_slip_geometry_mirror_kernel` used to perform at runtime; since the
    /// geometry is static, this only needs to run once.
    pub fn build(
        grid: &Grid,
        signed_distance_function_slip: &[Float],
        normals_slip_surfaces: &[SpatialVector],
        epsilon: Float,
        reach_distance: Float,
        order: SlipMirrorInterpolationOrder,
    ) -> Self {
        let mut entries: [Vec<SlipMirrorEntry>; 3] = Default::default();

        let [nxi, nyi, nzi] = grid.interior_shape;

        for ii in 0..nxi {
            for ji in 0..nyi {
                for ki in 0..nzi {
                    let extended_indices = grid.extended_indices_from_interior_indices([ii, ji, ki]);
                    let i_0 = grid.flat_index_on_extended_grid(extended_indices);

                    for axis_index in 0..3 {
                        let mut extended_indices_p = extended_indices;
                        extended_indices_p[axis_index] += 1;

                        let i_p = grid.flat_index_on_extended_grid(extended_indices_p);

                        let sdf = 0.5 * (
                            signed_distance_function_slip[i_0] +
                            signed_distance_function_slip[i_p]
                        );

                        // Fluid-side faces (sdf >= 0) must stay completely untouched (see the doc
                        // comment on the old mirror kernel), and cells deeper than
                        // `reach_distance` can never affect the flow outside the body — skip both
                        // by simply not creating an entry.
                        if sdf >= 0.0 || sdf <= -reach_distance {
                            continue;
                        }

                        let mu = Geometry::blending_function(sdf, epsilon);

                        let normal = (
                            0.5 * (normals_slip_surfaces[i_0] + normals_slip_surfaces[i_p])
                        ).normalize();

                        let mut face_center = grid.cell_center_extended(extended_indices);
                        face_center[axis_index] += 0.5 * grid.cell_length[axis_index];

                        // Reflect the face location across the (locally linear) interface to get
                        // the image point on the fluid side: `sdf` is negative inside the body, so
                        // this moves outward.
                        let image_point = face_center - 2.0 * sdf * normal;

                        let component_stencils: [SlipMirrorInterpolationStencil; 3] = std::array::from_fn(|component| {
                            let mut field_origin = grid.cell_center_extended([0, 0, 0]);
                            field_origin[component] += 0.5 * grid.cell_length[component];

                            match order {
                                SlipMirrorInterpolationOrder::Trilinear => SlipMirrorInterpolationStencil::Trilinear(
                                    grid.trilinear_stencil_at(field_origin, image_point)
                                ),
                                SlipMirrorInterpolationOrder::Tricubic => SlipMirrorInterpolationStencil::Tricubic(
                                    grid.tricubic_stencil_at(field_origin, image_point)
                                ),
                            }
                        });

                        entries[axis_index].push(SlipMirrorEntry {
                            cell_index: i_0,
                            mu,
                            normal,
                            component_stencils,
                        });
                    }
                }
            }
        }

        Self { entries }
    }
}
