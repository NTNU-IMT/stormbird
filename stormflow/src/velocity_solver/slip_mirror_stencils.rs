use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::grid::interpolation::{TrilinearStencil, TricubicStencil};
use crate::geometry::Geometry;

use rayon::prelude::*;

use super::no_slip_corrections::{NoSlipCorrections, NoSlipEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// Which interpolation order is used to sample the mirrored image point for the *velocity* mirror
/// correction specifically (the slip geometries, and the no-slip geometries with the wall model) —
/// independent of
/// `pressure_solver::multigrid_cpu::zero_gradient_stencils::ZeroGradientInterpolationOrder`, which
/// is the same choice for the pressure side. The two are deliberately separate settings (velocity
/// and pressure corrections live in unrelated solvers with no shared configuration today), but set
/// them to the same order if you want matching accuracy at the slip wall for both fields.
pub enum SlipMirrorInterpolationOrder {
    /// 2nd order accurate. Trilinear weights are always non-negative and sum to 1 (a true convex
    /// combination, unlike tricubic's), so this can never amplify oscillatory errors, and is the
    /// safer choice near thin/close-together walls, where the tricubic stencil's wider reach is
    /// more likely to pull in image points from the "wrong side" of a nearby second surface.
    /// Default.
    #[default]
    Trilinear,
    /// 4th order accurate, matching the rest of the solver's accuracy order.
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

/// Calls `face_action` for every staggered face (one cell, one axis) that gets a slip-mirror entry,
/// with the face's base cell extended indices, the flat extended indices of the two cells on each
/// side of the face, the axis, and the signed distance at the face. Shared by
/// `SlipMirrorStencils::build` and `SlipMirrorStencils::cells_needing_normals`, so that the normals
/// are always computed for exactly the cells the entries read them from.
fn for_each_corrected_face(
    grid: &Grid,
    signed_distance_function_slip: &[Float],
    reach_distance: Float,
    mut face_action: impl FnMut([usize; 3], usize, usize, usize, Float)
) {
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

                    face_action(extended_indices, i_0, i_p, axis_index, sdf);
                }
            }
        }
    }
}

impl SlipMirrorStencils {
    /// Returns, for every cell on the extended grid, whether `build` reads the slip surface normal
    /// of that cell, given the same `signed_distance_function_slip` and `reach_distance`. Used to
    /// only compute the normals where they are needed, as each normal is expensive to compute.
    pub fn cells_needing_normals(
        grid: &Grid,
        signed_distance_function_slip: &[Float],
        reach_distance: Float,
    ) -> Vec<bool> {
        let mut cells_needing_normals = vec![false; grid.nr_extended_cells()];

        for_each_corrected_face(
            grid, signed_distance_function_slip, reach_distance,
            |_, i_0, i_p, _, _| {
                cells_needing_normals[i_0] = true;
                cells_needing_normals[i_p] = true;
            }
        );

        cells_needing_normals
    }

    /// Builds the slip-mirror correction entries for every interior cell/axis within
    /// `reach_distance` of a slip surface, sampling the mirrored image point in the given
    /// interpolation `order`. Mirrors the per-cell geometry math that
    /// `correct_velocities_for_slip_geometry_mirror_kernel` used to perform at runtime; since the
    /// geometry is static, this only needs to run once. `normals_slip_surfaces` only needs to be
    /// valid for the cells marked by `cells_needing_normals`.
    pub fn build(
        grid: &Grid,
        signed_distance_function_slip: &[Float],
        normals_slip_surfaces: &[SpatialVector],
        epsilon: Float,
        reach_distance: Float,
        order: SlipMirrorInterpolationOrder,
    ) -> Self {
        let mut entries: [Vec<SlipMirrorEntry>; 3] = Default::default();

        for_each_corrected_face(
            grid, signed_distance_function_slip, reach_distance,
            |extended_indices, i_0, i_p, axis_index, sdf| {
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
        );

        Self { entries }
    }
}

/// How far inside the geometries, as a multiple of the largest cell length, the data immersion
/// that sets the velocity to zero in the interior of the mirror geometries is centered. With a
/// blending width of one cell, the velocity is zero beyond `SHIFT + 1` cells, and untouched closer
/// to the surface than `SHIFT - 1` cells (see `mirror_interior_corrections`).
pub const MIRROR_INTERIOR_SHIFT_CELLS: Float = 3.0;

/// How far inside the domain, as a multiple of the cell length along each axis, a mirrored image
/// point must be for the mirror correction to be used. Closer to the boundary, the interpolation
/// stencil reads the ghost cells, which, for instance at the ground, mirror the cells just inside
/// the geometry back to themselves.
pub const MIRROR_DOMAIN_MARGIN_CELLS: Float = 1.0;

/// Builds the corrections that set the velocity to zero inside the geometries with a mirror
/// correction (the slip geometries, and the no-slip geometries with the wall model), where the
/// mirror correction can not be used:
///
/// - Deep inside the geometries, beyond the reach of the mirror correction, with a blending
///   centered `MIRROR_INTERIOR_SHIFT_CELLS` cells inside the surface.
/// - Where the mirrored image point of a face is outside the domain, closer to the domain boundary
///   than `MIRROR_DOMAIN_MARGIN_CELLS`, or inside a geometry. This happens, e.g., for geometries
///   that extend to, through, or to less than a cell from the ground, as the cells just inside the
///   bottom of the geometry would otherwise be mirrored to themselves through the ghost cells, and
///   at concave corners and thin parts of the geometries.
///
/// `signed_distance_function` must be the signed distance function of the union of
/// `mirror_geometries`, while `all_geometries` are all geometries in the simulation. Returns the
/// corrections, together with the faces that are set fully to zero, as `(cell_index, axis)`, which
/// should not get any mirror correction.
pub fn mirror_interior_corrections(
    grid: &Grid,
    mirror_geometries: &[Geometry],
    all_geometries: &[Geometry],
    signed_distance_function: &[Float],
    mirror_reach_distance: Float,
) -> (NoSlipCorrections, Vec<(usize, usize)>) {
    let mut max_dx: Float = 0.0;
    for axis in 0..3 {
        max_dx = max_dx.max(grid.cell_length[axis]);
    }

    let shift = MIRROR_INTERIOR_SHIFT_CELLS * max_dx;
    let normal_delta = 0.1 * grid.cell_length;

    let domain_start = grid.start_point;
    let domain_end = grid.start_point + SpatialVector([
        grid.interior_shape[0] as Float * grid.cell_length[0],
        grid.interior_shape[1] as Float * grid.cell_length[1],
        grid.interior_shape[2] as Float * grid.cell_length[2],
    ]);

    let [nxi, nyi, nzi] = grid.interior_shape;

    let face_entries: Vec<(usize, NoSlipEntry)> = (0..nxi).into_par_iter().flat_map_iter(|ii| {
        let mut plane_entries = Vec::new();

        if mirror_geometries.is_empty() {
            return plane_entries;
        }

        for ji in 0..nyi {
            for ki in 0..nzi {
                let extended_indices = grid.extended_indices_from_interior_indices([ii, ji, ki]);
                let i_0 = grid.flat_index_on_extended_grid(extended_indices);

                for axis in 0..3 {
                    let i_p = i_0 + grid.extended_stride[axis];

                    let distance = 0.5 * (signed_distance_function[i_0] + signed_distance_function[i_p]);

                    if distance >= 0.0 {
                        continue;
                    }

                    let mut mu = Geometry::blending_function(distance + shift, max_dx);

                    if mu > 0.0 && distance > -mirror_reach_distance {
                        let mut face_center = grid.cell_center_extended(extended_indices);
                        face_center[axis] += 0.5 * grid.cell_length[axis];

                        let (_normal, image_point) = Geometry::mirror_image_point(
                            mirror_geometries, face_center, distance, normal_delta
                        );

                        let outside_domain = (0..3).any(|a| {
                            let margin = MIRROR_DOMAIN_MARGIN_CELLS * grid.cell_length[a];

                            image_point[a] < domain_start[a] + margin || image_point[a] > domain_end[a] - margin
                        });

                        let inside_geometry = Geometry::signed_distance_function_union(
                            all_geometries, image_point
                        ) < 0.0;

                        if outside_domain || inside_geometry {
                            mu = 0.0;
                        }
                    }

                    if mu < 1.0 {
                        plane_entries.push((axis, NoSlipEntry { cell_index: i_0, mu }));
                    }
                }
            }
        }

        plane_entries
    }).collect();

    let mut corrections = NoSlipCorrections::default();
    let mut zeroed_faces = Vec::new();

    for (axis, entry) in face_entries {
        if entry.mu == 0.0 {
            zeroed_faces.push((entry.cell_index, axis));
        }

        corrections.entries[axis].push(entry);
    }

    (corrections, zeroed_faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geometry::analytical_shapes::Cuboid;

    /// For a box standing on the ground, the faces just inside the bottom of the box have their
    /// mirror image below the ground, and must be set to zero rather than mirrored. Faces further
    /// up, close to the side walls, have valid mirror images.
    #[test]
    fn mirror_images_outside_the_domain_are_zeroed() {
        let grid = Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [16, 16, 16]);

        let geometries = [Geometry::Cuboid(Cuboid {
            center: SpatialVector([0.5, 0.5, 0.4]),
            half_extents: SpatialVector([0.25, 0.25, 0.4]),
        })];

        let sdf = Geometry::signed_distance_function_on_extended_grid(&geometries, &grid);

        let (corrections, zeroed_faces) = mirror_interior_corrections(&grid, &geometries, &geometries, &sdf, 3.0 / 16.0);

        let x_face = |indices: [usize; 3]| (grid.flat_index_on_extended_grid_from_interior_indices(indices), 0);

        // Just above the ground, in the middle of the box: mirrored to below the ground
        assert!(zeroed_faces.contains(&x_face([8, 8, 0])));

        // Close to the side wall, but well above the ground: a valid mirror image
        assert!(!zeroed_faces.contains(&x_face([4, 8, 4])));

        assert!(!corrections.entries[0].is_empty());
    }
}
