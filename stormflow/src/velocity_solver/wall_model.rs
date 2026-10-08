//! Wall model for the no-slip geometries, used instead of the data immersion when
//! `NoSlipWallTreatment::WallModel` is selected.
//!
//! The no-slip geometries are then treated in two parts:
//!
//! 1. The velocity inside the geometries is set with the same mirror (ghost cell) correction as the
//!    slip walls, so that the flow does not go through the surface, but slips along it. Further
//!    inside the geometries, beyond the reach of the mirror correction, the velocity is set to zero
//!    with the data immersion (see `slip_mirror_stencils::mirror_interior_corrections`).
//! 2. The wall shear stress from the log-law is added as a momentum sink in a thin band of fluid
//!    cells next to the surface, distributed with a smoothed delta function of the wall distance.
//!    The tangential velocity used in the log-law is sampled at a reference point outside the band.
//!
//! This is the same idea as wall functions in conventional RANS solvers, where the first cell
//! above the wall is in the log-law region, and the wall shear stress is imposed directly, rather
//! than through the velocity gradient at the wall. See the README for the details.

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;
use crate::grid::interpolation::TrilinearStencil;
use crate::geometry::Geometry;
use crate::log_law::WallFunctionConstants;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// How the velocity solver treats the no-slip geometries
pub enum NoSlipWallTreatment {
    /// The velocity is blended towards zero in a band of a few cells around the surface. Robust,
    /// also for thin geometries, but the wall shear stress depends on the grid resolution.
    /// Default.
    #[default]
    DataImmersion,
    /// A mirror (slip) correction inside the geometries, together with the wall shear stress from
    /// the log-law, applied as a momentum sink close to the surface. Requires geometries that are
    /// at least a few cells thick.
    WallModel,
}

/// The width of the band where the wall shear stress is applied, as a multiple of the largest cell
/// length. The delta function is a hat function centered at half the width.
pub const WALL_STRESS_BAND_CELLS: Float = 2.0;

/// The distance from the wall to the reference point where the velocity is sampled, as a multiple
/// of the largest cell length. Outside the band where the wall shear stress is applied.
pub const WALL_STRESS_REFERENCE_CELLS: Float = 2.5;

#[derive(Debug, Clone, Copy)]
/// The wall shear stress for one staggered velocity face (one cell, one axis)
pub struct WallStressEntry {
    /// Flat extended-grid index of the face's base cell
    pub cell_index: usize,
    /// The velocity component, i.e., the axis of the face
    pub axis: usize,
    /// The value of the smoothed delta function at the face, which distributes the wall shear
    /// stress, given per unit wall area, over the cells close to the wall.
    pub delta: Float,
    /// `1 - n[axis]^2`, which removes the wall-normal part of the stress for the component
    pub tangential_factor: Float,
    /// The wall normal, pointing into the fluid
    pub normal: SpatialVector,
    /// Interpolation stencils of the velocity at the reference point, one for each (staggered)
    /// velocity component
    pub velocity_stencils: [TrilinearStencil; 3],
}

#[derive(Debug, Clone, Default)]
/// Sparse list of the velocity faces where the wall shear stress is applied
pub struct WallStressEntries {
    pub entries: Vec<WallStressEntry>,
    /// The distance from the wall to the reference points
    pub reference_distance: Float,
    pub constants: WallFunctionConstants,
}

impl WallStressEntries {
    /// Builds the entries for all faces with a wall distance between zero and the band width,
    /// based on the signed distance function of the no-slip geometries. Faces inside a slip
    /// geometry get no entry.
    pub fn build(
        grid: &Grid,
        no_slip_geometries: &[Geometry],
        signed_distance_function: &[Float],
        signed_distance_function_slip: &[Float],
    ) -> Self {
        let mut max_dx: Float = 0.0;
        for axis in 0..3 {
            max_dx = max_dx.max(grid.cell_length[axis]);
        }

        let band_width = WALL_STRESS_BAND_CELLS * max_dx;
        let reference_distance = WALL_STRESS_REFERENCE_CELLS * max_dx;

        // Hat function centered in the band, with an integral of one over the wall distance
        let half_width = 0.5 * band_width;
        let delta_function = |distance: Float| {
            (1.0 - (distance - half_width).abs() / half_width).max(0.0) / half_width
        };

        let normal_delta = 0.1 * grid.cell_length;
        let extended_origin = grid.cell_center_extended([0, 0, 0]);

        let [nxi, nyi, nzi] = grid.interior_shape;

        if no_slip_geometries.is_empty() {
            return Self {
                entries: Vec::new(),
                reference_distance,
                constants: WallFunctionConstants::default(),
            };
        }

        // Parallel over x-planes, with the entries of each plane in the same order as a sequential
        // loop over the cells, so the result is deterministic
        let entries: Vec<WallStressEntry> = (0..nxi).into_par_iter().flat_map_iter(|ii| {
            let mut plane_entries = Vec::new();

            for ji in 0..nyi {
                for ki in 0..nzi {
                    let extended_indices = grid.extended_indices_from_interior_indices([ii, ji, ki]);
                    let i_0 = grid.flat_index_on_extended_grid(extended_indices);

                    for axis in 0..3 {
                        let i_p = i_0 + grid.extended_stride[axis];

                        let distance = 0.5 * (signed_distance_function[i_0] + signed_distance_function[i_p]);
                        let distance_slip = 0.5 * (signed_distance_function_slip[i_0] + signed_distance_function_slip[i_p]);

                        if distance <= 0.0 || distance >= band_width || distance_slip < 0.0 {
                            continue;
                        }

                        let mut face_center = grid.cell_center_extended(extended_indices);
                        face_center[axis] += 0.5 * grid.cell_length[axis];

                        let normal = Geometry::normal_from_signed_distance_function_union(
                            no_slip_geometries, face_center, normal_delta
                        );

                        let reference_point = face_center + (reference_distance - distance) * normal;

                        let velocity_stencils: [TrilinearStencil; 3] = std::array::from_fn(|component| {
                            let mut field_origin = extended_origin;
                            field_origin[component] += 0.5 * grid.cell_length[component];

                            grid.trilinear_stencil_at(field_origin, reference_point)
                        });

                        plane_entries.push(WallStressEntry {
                            cell_index: i_0,
                            axis,
                            delta: delta_function(distance),
                            tangential_factor: 1.0 - normal[axis] * normal[axis],
                            normal,
                            velocity_stencils,
                        });
                    }
                }
            }

            plane_entries
        }).collect();

        Self {
            entries,
            reference_distance,
            constants: WallFunctionConstants::default(),
        }
    }
}

#[inline(always)]
/// Applies the wall shear stress to the `current` value of the face of `entry` in the predicted
/// velocity. The magnitude of the stress, `tau_w = u_tau^2`, comes from the log-law for the
/// tangential velocity at the reference point, sampled from `velocity`. The stress is applied
/// point-implicitly, as a sink that is linear in the local velocity, `-tau_w delta u / |U_t|`,
/// which equals the full wall shear stress when the local velocity is parallel to, and as large
/// as, the reference velocity. This makes the sink unconditionally stable, and it can never
/// reverse the flow.
pub fn wall_stress_kernel(
    entry: &WallStressEntry,
    reference_distance: Float,
    constants: &WallFunctionConstants,
    y_plus_lam: Float,
    velocity: &[SpatialVector],
    stride: [usize; 3],
    viscosity: Float,
    time_step: Float,
    current: Float,
) -> Float {
    let mut v = SpatialVector::default();

    for component in 0..3 {
        v[component] = entry.velocity_stencils[component].sample_component(velocity, component, stride);
    }

    let normal_velocity = v.dot(entry.normal);
    let tangential_velocity = (v - normal_velocity * entry.normal).length();

    if tangential_velocity < 1e-12 {
        return current;
    }

    let friction_velocity = constants.friction_velocity(
        tangential_velocity, reference_distance, viscosity, y_plus_lam
    );

    let sink_coefficient = friction_velocity * friction_velocity * entry.delta / tangential_velocity;

    current / (1.0 + time_step * sink_coefficient * entry.tangential_factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geometry::analytical_shapes::Cuboid;

    /// For a flat wall aligned with the grid, the delta function sums to one over the faces in a
    /// column normal to the wall, so that the total force per unit area is the wall shear stress.
    #[test]
    fn delta_function_integrates_to_one_for_aligned_wall() {
        let grid = Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [16, 16, 16]);

        for wall_height in [0.3, 0.3125, 0.33] {
            // A large box below the plane z = wall_height
            let geometry = Geometry::Cuboid(Cuboid {
                center: SpatialVector([0.5, 0.5, wall_height - 5.0]),
                half_extents: SpatialVector([10.0, 10.0, 5.0]),
            });

            let geometries = [geometry];

            let sdf = Geometry::signed_distance_function_on_extended_grid(&geometries, &grid);
            let sdf_slip = vec![Float::MAX; grid.nr_extended_cells()];

            let walls = WallStressEntries::build(&grid, &geometries, &sdf, &sdf_slip);

            let column = grid.extended_indices_from_interior_indices([5, 7, 0]);

            // The x-velocity faces in one column along z
            let sum: Float = walls.entries.iter()
                .filter(|entry| {
                    let indices = grid.extended_indices_from_flat_index(entry.cell_index);

                    entry.axis == 0 && indices[0] == column[0] && indices[1] == column[1]
                })
                .map(|entry| {
                    assert!((entry.tangential_factor - 1.0).abs() < 1e-4);
                    entry.delta * grid.cell_length[2]
                })
                .sum();

            assert!((sum - 1.0).abs() < 1e-3, "wall at {wall_height}: {sum}");

            // The wall-normal faces get no stress
            assert!(walls.entries.iter().filter(|entry| entry.axis == 2).all(|entry| entry.tangential_factor < 1e-4));
        }
    }
}
