use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::grid::interpolation::{TrilinearStencil, TricubicStencil};
use crate::geometry::Geometry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// Which interpolation order is used to sample the mirrored image point for the slip-wall pressure
/// correction specifically — independent of the rest of the pressure solve, which always uses the
/// 4th order stencils in `kernels::jacobi`/`kernels::coarse_matrix` regardless of this setting.
pub enum SlipPressureInterpolationOrder {
    /// 2nd order accurate. Trilinear weights are always non-negative and sum to 1 (a true convex
    /// combination), so this interpolation can never amplify oscillatory error the way the cubic
    /// variant can (see `SLIP_CORRECTION_RELAXATION`'s doc comment) — useful for comparing whether
    /// that's actually what's driving a given stability/accuracy trade-off.
    Trilinear,
    /// 4th order accurate, matching the rest of the pressure solve's accuracy order. Default.
    #[default]
    Tricubic,
}

#[derive(Debug, Clone, Copy)]
/// The interpolation stencil for one `SlipPressureEntry`'s mirrored image point, in whichever order
/// `SlipPressureInterpolationOrder` selected when the entries were built.
pub enum SlipPressureInterpolationStencil {
    Trilinear(TrilinearStencil),
    Tricubic(TricubicStencil),
}

impl SlipPressureInterpolationStencil {
    #[inline(always)]
    pub fn sample_scalar(&self, field: &[Float], stride: [usize; 3]) -> Float {
        match self {
            Self::Trilinear(stencil) => stencil.sample_scalar(field, stride),
            Self::Tricubic(stencil) => stencil.sample_scalar(field, stride),
        }
    }
}

/// How many grid cells deep into a slip body the pressure zero-gradient correction is still
/// computed, expressed as a multiple of the largest cell length *of the level it's built for*
/// (each multigrid level has its own cell size, so this is re-evaluated per level). Matches the
/// pressure operator's own stencil half-width (`off_diagonal_sum`/`laplacian_stencil4` reach 2
/// cells), unlike velocity's 3-cell reach from the upwind advection term — a body cell any deeper
/// than this can never be read by a real fluid cell's pressure stencil at that level. Also used
/// directly as the blending width (`epsilon`), so the blend saturates to fully-mirrored exactly at
/// the pruning boundary instead of leaving a partially-blended band that then gets discarded.
pub const SLIP_PRESSURE_REACH_CELLS: Float = 2.0;

#[derive(Debug, Clone)]
/// A precomputed pressure zero-gradient (Neumann) correction for one interior cell of one
/// multigrid level. Everything geometry-dependent (how much to blend, the interpolation stencil
/// for sampling the mirrored image point) is computed once, since the slip geometry is static;
/// only the pressure value itself is re-sampled every time the correction is applied. Which cell
/// an entry belongs to isn't stored here — `SlipPressureStencils::cell_lookup` maps a cell's own
/// flat index directly to its entry, so `jacobi_kernel_with_slip_correction` never needs to search.
pub struct SlipPressureEntry {
    /// Blend factor between the cell's own pressure and the mirrored image-point pressure: `0`
    /// deep inside the body, `1` in the fluid (such cells get no entry at all).
    pub mu: Float,
    /// Interpolation stencil for sampling the mirrored image point's pressure, indexed on the
    /// interior-grid layout (see `Grid::tricubic_stencil_at_interior`/`trilinear_stencil_at_interior`).
    pub stencil: SlipPressureInterpolationStencil,
}

#[derive(Debug, Clone, Default)]
pub struct SlipPressureStencils {
    pub entries: Vec<SlipPressureEntry>,
    /// Per interior cell (flat index, same length as `MultigridCPU::x_at_levels` at this level
    /// when non-empty): `-1` if the cell is uncorrected, otherwise the index into `entries` for
    /// that cell's correction. Lets `jacobi_kernel_with_slip_correction` do an O(1) lookup per cell
    /// (one cheap, sequentially-accessed, branch-predictable array read) instead of needing a
    /// separate pass over `entries`. Empty (not the all-`-1` array) when there are no slip
    /// geometries, matching `entries`.
    pub cell_lookup: Vec<i32>,
}

impl SlipPressureStencils {
    /// Builds the pressure zero-gradient correction entries for `grid` (one level of a multigrid
    /// hierarchy), `slip_geometries`, and the chosen interpolation `order`. The signed distance
    /// function and normals are only needed transiently here to build the stencils, not kept around
    /// afterward — everything needed at solve time ends up baked into the returned
    /// `entries`/`cell_lookup`.
    pub fn build(grid: &Grid, slip_geometries: &[Geometry], order: SlipPressureInterpolationOrder) -> Self {
        if slip_geometries.is_empty() {
            return Self::default();
        }

        let signed_distance_function_slip = Geometry::signed_distance_function_on_extended_grid(
            slip_geometries, grid
        );
        let normals_slip_surfaces = Geometry::geometry_normals_on_extended_grid(
            slip_geometries, grid, 0.1
        );

        let mut max_dx = 0.0;
        for axis_index in 0..3 {
            if grid.cell_length[axis_index] > max_dx {
                max_dx = grid.cell_length[axis_index];
            }
        }

        // The reach also doubles as the blending width, so `mu` saturates to 0 exactly at the
        // pruning boundary (see `SLIP_PRESSURE_REACH_CELLS`'s doc comment).
        let epsilon = SLIP_PRESSURE_REACH_CELLS * max_dx;
        let reach_distance = epsilon;

        let field_origin = grid.cell_center([0, 0, 0]);

        let mut entries = Vec::new();
        let mut cell_lookup = vec![-1i32; grid.nr_interior_cells()];

        let [nxi, nyi, nzi] = grid.interior_shape;

        for ii in 0..nxi {
            for ji in 0..nyi {
                for ki in 0..nzi {
                    let extended_indices = grid.extended_indices_from_interior_indices([ii, ji, ki]);
                    let i_extended = grid.flat_index_on_extended_grid(extended_indices);

                    let sdf = signed_distance_function_slip[i_extended];

                    // Fluid-side cells (sdf >= 0) must stay untouched, and cells deeper than
                    // `reach_distance` can never affect the pressure field outside the body at
                    // this level's resolution — skip both by simply not creating an entry.
                    if sdf >= 0.0 || sdf <= -reach_distance {
                        continue;
                    }

                    let mu = Geometry::blending_function(sdf, epsilon);
                    let normal = normals_slip_surfaces[i_extended];

                    let cell_center = grid.cell_center_extended(extended_indices);

                    // Reflect the cell center across the (locally linear) interface to get the
                    // image point on the fluid side: `sdf` is negative inside the body, so this
                    // moves outward.
                    let image_point = cell_center - 2.0 * sdf * normal;

                    let stencil = match order {
                        SlipPressureInterpolationOrder::Trilinear => SlipPressureInterpolationStencil::Trilinear(
                            grid.trilinear_stencil_at_interior(field_origin, image_point)
                        ),
                        SlipPressureInterpolationOrder::Tricubic => SlipPressureInterpolationStencil::Tricubic(
                            grid.tricubic_stencil_at_interior(field_origin, image_point)
                        ),
                    };

                    let cell_index = grid.flat_index_on_interior_grid([ii, ji, ki]);
                    cell_lookup[cell_index] = entries.len() as i32;

                    entries.push(SlipPressureEntry {
                        mu,
                        stencil,
                    });
                }
            }
        }

        Self { entries, cell_lookup }
    }
}
