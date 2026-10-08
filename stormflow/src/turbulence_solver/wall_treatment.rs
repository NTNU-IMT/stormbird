//! The simplified treatment of the wall geometries in the turbulence models. Everything here is
//! precomputed once, when the simulation is built, as sparse lists of entries, in the same way as
//! the geometry corrections of the velocity solver. Three different corrections are used:
//!
//! - **Mirror entries**: all cells inside a geometry (slip or no-slip), up to a few cells deep, get
//!   the values of the transported fields at their mirrored image point across the surface. This
//!   gives a zero gradient condition on the transported fields at all walls, which is also what
//!   OpenFOAM's wall functions use for `k`.
//! - **Wall function entries**: the fluid cells close to a no-slip geometry, where the velocity is
//!   damped by the no-slip correction, get the production of turbulent kinetic energy and the
//!   wall-normal variable (e.g., epsilon) from standard wall functions. The wall distance is taken
//!   from the signed distance function, and the velocity used for the wall shear is sampled
//!   further out, outside the damped region. Without this, the large velocity gradients created
//!   by the no-slip correction would produce unphysical amounts of turbulence.
//! - **Damping entries**: the eddy viscosity is blended towards zero inside the no-slip geometries,
//!   with the same blending function as the no-slip velocity correction.

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;
use crate::grid::interpolation::TrilinearStencil;
use crate::geometry::{Geometry, WallGeometries};

/// How many cells deep into a geometry the mirror entries are computed, as a multiple of the
/// largest cell length. The widest stencil that reads the transported fields is the limited linear
/// convection, which reaches two cells, so this has some margin.
pub const MIRROR_REACH_CELLS: Float = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// How the turbulence model treats the no-slip walls. The slip walls always use a zero gradient
/// condition.
pub enum WallTreatment {
    /// Only a zero gradient condition on the transported fields, with no special treatment of the
    /// cells close to the wall.
    ZeroGradient,
    /// Zero gradient inside the geometries, and standard wall functions in the fluid cells close to
    /// the wall. Default.
    #[default]
    WallFunction,
}

#[derive(Debug, Clone, Copy)]
/// The constants of the standard log-law wall functions, with the same default values as in
/// OpenFOAM.
pub struct WallFunctionConstants {
    pub c_mu: Float,
    pub kappa: Float,
    pub e: Float,
}

impl Default for WallFunctionConstants {
    fn default() -> Self {
        Self {
            c_mu: 0.09,
            kappa: 0.41,
            e: 9.8,
        }
    }
}

impl WallFunctionConstants {
    /// The y+ value where the viscous sublayer and the log-law region intersect, computed in the
    /// same way as in OpenFOAM's `nutWallFunction`.
    pub fn y_plus_lam(&self) -> Float {
        let mut y_plus_lam: Float = 11.0;

        for _ in 0..10 {
            y_plus_lam = (self.e * y_plus_lam).max(1.0).ln() / self.kappa;
        }

        y_plus_lam
    }
}

#[derive(Debug, Clone, Copy)]
/// Zero gradient correction of the transported fields in one cell inside a geometry
pub struct MirrorEntry {
    /// Flat extended-grid index of the corrected cell
    pub cell_index: usize,
    /// Interpolation stencil of the mirrored image point, on the extended grid. Trilinear, so that
    /// the interpolated values are always positive.
    pub stencil: TrilinearStencil,
}

#[derive(Debug, Clone, Copy)]
/// Wall function for one fluid cell close to a no-slip geometry
pub struct WallFunctionEntry {
    /// Flat extended-grid index of the cell
    pub cell_index: usize,
    /// The distance from the cell center to the wall used in the wall function
    pub wall_distance: Float,
    /// The distance from the wall to the point where the velocity is sampled
    pub reference_distance: Float,
    /// The wall normal, pointing into the fluid
    pub normal: SpatialVector,
    /// Interpolation stencils of the velocity at the reference point, one for each (staggered)
    /// velocity component
    pub velocity_stencils: [TrilinearStencil; 3],
}

impl WallFunctionEntry {
    #[inline(always)]
    /// The magnitude of the velocity tangential to the wall at the reference point
    pub fn tangential_velocity(&self, velocity: &[SpatialVector], stride: [usize; 3]) -> Float {
        let mut v = SpatialVector::default();

        for component in 0..3 {
            v[component] = self.velocity_stencils[component].sample_component(velocity, component, stride);
        }

        let normal_velocity = v.dot(self.normal);

        (v - normal_velocity * self.normal).length()
    }
}

#[derive(Debug, Clone, Copy)]
/// Blending of the eddy viscosity towards zero in one cell close to or inside a no-slip geometry
pub struct DampingEntry {
    /// Flat extended-grid index of the cell
    pub cell_index: usize,
    /// The blending factor: 0 inside the geometry, 1 in the fluid (such cells get no entry)
    pub mu: Float,
}

#[derive(Debug, Clone, Default)]
/// All the precomputed wall corrections of the turbulence solver
pub struct WallTreatmentEntries {
    pub mirror: Vec<MirrorEntry>,
    pub wall_functions: Vec<WallFunctionEntry>,
    pub damping: Vec<DampingEntry>,
}

fn cell_lengths(grid: &Grid) -> (Float, Float) {
    let mut min_dx = Float::MAX;
    let mut max_dx: Float = 0.0;

    for axis in 0..3 {
        min_dx = min_dx.min(grid.cell_length[axis]);
        max_dx = max_dx.max(grid.cell_length[axis]);
    }

    (min_dx, max_dx)
}

/// The flat extended indices of all interior cells
fn interior_cells(grid: &Grid) -> impl ParallelIterator<Item = usize> + '_ {
    (0..grid.nr_interior_cells()).into_par_iter().map(|interior_index| {
        grid.flat_index_on_extended_grid_from_interior_indices(
            grid.interior_indices_from_flat_index(interior_index)
        )
    })
}

impl WallTreatmentEntries {
    /// Builds the entries for the given geometries. `no_slip_epsilon` must be the blending width
    /// used by the no-slip velocity correction (see `NoSlipCorrections::build`).
    pub fn build(
        grid: &Grid,
        no_slip_walls: &WallGeometries,
        slip_walls: &WallGeometries,
        no_slip_epsilon: Float,
        wall_treatment: WallTreatment,
    ) -> Self {
        let (min_dx, max_dx) = cell_lengths(grid);
        let normal_delta = 0.1 * grid.cell_length;

        let has_no_slip = !no_slip_walls.geometries.is_empty();
        let has_slip = !slip_walls.geometries.is_empty();

        let no_slip_sdf = |i: usize| if has_no_slip {
            no_slip_walls.signed_distance_function[i]
        } else {
            Float::MAX
        };

        let slip_sdf = |i: usize| if has_slip {
            slip_walls.signed_distance_function[i]
        } else {
            Float::MAX
        };

        // --- Mirror entries, for all geometries ---
        let all_geometries: Vec<Geometry> = no_slip_walls.geometries.iter()
            .chain(slip_walls.geometries.iter())
            .cloned()
            .collect();

        let mirror_reach = MIRROR_REACH_CELLS * max_dx;
        let extended_origin = grid.cell_center_extended([0, 0, 0]);

        let mut mirror: Vec<MirrorEntry> = interior_cells(grid).filter_map(|cell_index| {
            let sdf = no_slip_sdf(cell_index).min(slip_sdf(cell_index));

            if sdf >= 0.0 || sdf <= -mirror_reach {
                return None;
            }

            let cell_center = grid.cell_center_extended(grid.extended_indices_from_flat_index(cell_index));

            let (_normal, image_point) = Geometry::mirror_image_point(
                &all_geometries, cell_center, sdf, normal_delta
            );

            Some(MirrorEntry {
                cell_index,
                stencil: grid.trilinear_stencil_at(extended_origin, image_point),
            })
        }).collect();

        // --- Wall function entries, for the no-slip geometries ---
        // The velocity is damped by the no-slip correction up to `no_slip_epsilon` from the wall,
        // so the wall shear is computed from the velocity half a cell further out.
        let reference_distance = no_slip_epsilon + 0.5 * max_dx;
        let minimum_wall_distance = 0.5 * min_dx;

        let mut wall_functions: Vec<WallFunctionEntry> = if has_no_slip && wall_treatment == WallTreatment::WallFunction {
            interior_cells(grid).filter_map(|cell_index| {
                let sdf = no_slip_sdf(cell_index);

                if sdf < 0.0 || sdf >= no_slip_epsilon || slip_sdf(cell_index) < 0.0 {
                    return None;
                }

                let cell_center = grid.cell_center_extended(grid.extended_indices_from_flat_index(cell_index));

                let normal = Geometry::normal_from_signed_distance_function_union(
                    &no_slip_walls.geometries, cell_center, normal_delta
                );

                let reference_point = cell_center + (reference_distance - sdf) * normal;

                let velocity_stencils: [TrilinearStencil; 3] = std::array::from_fn(|component| {
                    let mut field_origin = extended_origin;
                    field_origin[component] += 0.5 * grid.cell_length[component];

                    grid.trilinear_stencil_at(field_origin, reference_point)
                });

                Some(WallFunctionEntry {
                    cell_index,
                    wall_distance: sdf.max(minimum_wall_distance),
                    reference_distance,
                    normal,
                    velocity_stencils,
                })
            }).collect()
        } else {
            Vec::new()
        };

        // --- Damping of the eddy viscosity, for the no-slip geometries ---
        let mut damping: Vec<DampingEntry> = if has_no_slip {
            interior_cells(grid).filter_map(|cell_index| {
                let mu = Geometry::blending_function(no_slip_sdf(cell_index), no_slip_epsilon);

                (mu < 1.0).then_some(DampingEntry { cell_index, mu })
            }).collect()
        } else {
            Vec::new()
        };

        // Sorted, so that the memory access is as sequential as possible
        mirror.sort_by_key(|entry| entry.cell_index);
        wall_functions.sort_by_key(|entry| entry.cell_index);
        damping.sort_by_key(|entry| entry.cell_index);

        Self {
            mirror,
            wall_functions,
            damping,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y_plus_lam_matches_openfoam() {
        let y_plus_lam = WallFunctionConstants::default().y_plus_lam();

        assert!((y_plus_lam - 11.53).abs() < 0.01, "{y_plus_lam}");
    }
}
