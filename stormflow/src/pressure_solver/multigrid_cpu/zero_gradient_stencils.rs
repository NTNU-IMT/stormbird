use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::grid::interpolation::{TrilinearStencil, TricubicStencil};
use crate::geometry::Geometry;

use super::kernels::jacobi::ZERO_GRADIENT_RELAXATION;
use super::settings::MultigridSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
/// Which interpolation order is used to sample the mirrored image point for the zero-gradient
/// condition on walls specifically — independent of the rest of the pressure solve, which always
/// uses the 4th order stencils in `kernels::jacobi`/`kernels::coarse_matrix` regardless of this
/// setting.
pub enum ZeroGradientInterpolationOrder {
    /// 2nd order accurate. Trilinear weights are always non-negative and sum to 1 (a true convex
    /// combination), so this interpolation can never amplify oscillatory error the way the cubic
    /// variant can (see `ZERO_GRADIENT_RELAXATION`'s doc comment) — useful for comparing whether
    /// that's actually what's driving a given stability/accuracy trade-off.
    Trilinear,
    /// 4th order accurate, matching the rest of the pressure solve's accuracy order. Default.
    #[default]
    Tricubic,
}

#[derive(Debug, Clone, Copy)]
/// The interpolation stencil for one `ZeroGradientEntry`'s mirrored image point, in whichever order
/// `ZeroGradientInterpolationOrder` selected when the entries were built.
pub enum ZeroGradientInterpolationStencil {
    Trilinear(TrilinearStencil),
    Tricubic(TricubicStencil),
}

impl ZeroGradientInterpolationStencil {
    #[inline(always)]
    pub fn sample_scalar(&self, field: &[Float], stride: [usize; 3]) -> Float {
        match self {
            Self::Trilinear(stencil) => stencil.sample_scalar(field, stride),
            Self::Tricubic(stencil) => stencil.sample_scalar(field, stride),
        }
    }
}

/// How many grid cells deep into a wall geometry the pressure zero-gradient correction is still
/// computed, expressed as a multiple of the largest cell length *of the level it's built for*
/// (each multigrid level has its own cell size, so this is re-evaluated per level). Matches the
/// pressure operator's own stencil half-width (`off_diagonal_sum`/`laplacian_stencil4` reach 2
/// cells), unlike velocity's 3-cell reach from the upwind advection term — a body cell any deeper
/// than this can never be read by a real fluid cell's pressure stencil at that level. Also used
/// directly as the blending width (`epsilon`), so the blend saturates to fully-mirrored exactly at
/// the pruning boundary instead of leaving a partially-blended band that then gets discarded.
pub const ZERO_GRADIENT_REACH_CELLS: Float = 2.0;

#[derive(Debug, Clone)]
/// A precomputed pressure zero-gradient (Neumann) correction for one interior cell of one
/// multigrid level. Everything geometry-dependent (how much to blend, the interpolation stencil
/// for sampling the mirrored image point) is computed once, since the geometry is static;
/// only the pressure value itself is re-sampled every time the correction is applied. Which cell
/// an entry belongs to isn't stored here — `ZeroGradientStencils::cell_lookup` maps a cell's own
/// flat index directly to its entry, so `jacobi_kernel_with_zero_gradient` never needs to search.
pub struct ZeroGradientEntry {
    /// Blend factor between the cell's own pressure and the mirrored image-point pressure: `0`
    /// deep inside the body, `1` in the fluid (such cells get no entry at all).
    pub mu: Float,
    /// Interpolation stencil for sampling the mirrored image point's pressure, indexed on the
    /// interior-grid layout (see `Grid::tricubic_stencil_at_interior`/`trilinear_stencil_at_interior`).
    pub stencil: ZeroGradientInterpolationStencil,
}

#[derive(Debug, Clone, Default)]
pub struct ZeroGradientStencils {
    pub entries: Vec<ZeroGradientEntry>,
    /// Per interior cell (flat index, same length as `MultigridCPU::x_at_levels` at this level
    /// when non-empty): `-1` if the cell is uncorrected, otherwise the index into `entries` for
    /// that cell's correction. Lets `jacobi_kernel_with_zero_gradient` do an O(1) lookup per cell
    /// (one cheap, sequentially-accessed, branch-predictable array read) instead of needing a
    /// separate pass over `entries`. Empty (not the all-`-1` array) when there are no wall
    /// geometries, matching `entries`.
    pub cell_lookup: Vec<i32>,
}

impl ZeroGradientStencils {
    /// Builds the stencils for every level in `grids` (a multigrid hierarchy), for the walls
    /// selected by `settings.zero_gradient_on_walls`. All levels get empty stencils if the
    /// condition is not used. Shared by `MultigridCPU` and `MultigridGPU`.
    pub fn build_for_all_levels(
        grids: &[Grid],
        settings: &MultigridSettings,
        slip_geometries: &[Geometry],
        no_slip_geometries: &[Geometry]
    ) -> Vec<Self> {
        let wall_geometries = settings.zero_gradient_on_walls.wall_geometries(
            slip_geometries, no_slip_geometries
        );

        if !wall_geometries.is_empty() {
            println!("Building per-level zero-gradient stencils");
        }

        grids.iter()
            .map(|level_grid| Self::build(level_grid, &wall_geometries, settings.zero_gradient_interpolation_order))
            .collect()
    }

    /// Builds the pressure zero-gradient correction entries for `grid` (one level of a multigrid
    /// hierarchy), `wall_geometries`, and the chosen interpolation `order`. The signed distance
    /// function and normals are only needed transiently here to build the stencils, not kept around
    /// afterward — everything needed at solve time ends up baked into the returned
    /// `entries`/`cell_lookup`.
    pub fn build(grid: &Grid, wall_geometries: &[Geometry], order: ZeroGradientInterpolationOrder) -> Self {
        if wall_geometries.is_empty() {
            return Self::default();
        }

        let signed_distance_function = Geometry::signed_distance_function_on_extended_grid(
            wall_geometries, grid
        );
        let normals = Geometry::geometry_normals_on_extended_grid(
            wall_geometries, grid, 0.1
        );

        let mut max_dx = 0.0;
        for axis_index in 0..3 {
            if grid.cell_length[axis_index] > max_dx {
                max_dx = grid.cell_length[axis_index];
            }
        }

        // The reach also doubles as the blending width, so `mu` saturates to 0 exactly at the
        // pruning boundary (see `ZERO_GRADIENT_REACH_CELLS`'s doc comment).
        let epsilon = ZERO_GRADIENT_REACH_CELLS * max_dx;
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

                    let sdf = signed_distance_function[i_extended];

                    // Fluid-side cells (sdf >= 0) must stay untouched, and cells deeper than
                    // `reach_distance` can never affect the pressure field outside the body at
                    // this level's resolution — skip both by simply not creating an entry.
                    if sdf >= 0.0 || sdf <= -reach_distance {
                        continue;
                    }

                    let mu = Geometry::blending_function(sdf, epsilon);
                    let normal = normals[i_extended];

                    let cell_center = grid.cell_center_extended(extended_indices);

                    // Reflect the cell center across the (locally linear) interface to get the
                    // image point on the fluid side: `sdf` is negative inside the body, so this
                    // moves outward.
                    let image_point = cell_center - 2.0 * sdf * normal;

                    let stencil = match order {
                        ZeroGradientInterpolationOrder::Trilinear => ZeroGradientInterpolationStencil::Trilinear(
                            grid.trilinear_stencil_at_interior(field_origin, image_point)
                        ),
                        ZeroGradientInterpolationOrder::Tricubic => ZeroGradientInterpolationStencil::Tricubic(
                            grid.tricubic_stencil_at_interior(field_origin, image_point)
                        ),
                    };

                    let cell_index = grid.flat_index_on_interior_grid([ii, ji, ki]);
                    cell_lookup[cell_index] = entries.len() as i32;

                    entries.push(ZeroGradientEntry {
                        mu,
                        stencil,
                    });
                }
            }
        }

        Self { entries, cell_lookup }
    }
}

/// Applies the zero-gradient correction on walls to `x` once, in place, sequentially — for solve
/// paths that aren't the per-sweep fused Jacobi kernel: `MultigridCPU`'s coarsest-level exact (Gaussian
/// elimination) solve, and `MultigridGPU`'s equivalent (which already round-trips that level's
/// solution through the host to run the same Gaussian elimination on the CPU). Under-relaxed by
/// `ZERO_GRADIENT_RELAXATION`, matching `jacobi_kernel_with_zero_gradient`, for consistency —
/// though a single one-shot application is far less exposed to the compounding-amplification issue
/// that relaxation was actually introduced to fix.
pub(crate) fn apply_relaxed_correction(x: &mut [Float], stencils: &ZeroGradientStencils, stride: [usize; 3]) {
    if stencils.entries.is_empty() {
        return;
    }

    let x_snapshot = x.to_vec();

    for (idx, &lookup_index) in stencils.cell_lookup.iter().enumerate() {
        if lookup_index < 0 {
            continue;
        }

        let entry = &stencils.entries[lookup_index as usize];
        let p_image = entry.stencil.sample_scalar(&x_snapshot, stride);
        let corrected_target = entry.mu * x_snapshot[idx] + (1.0 - entry.mu) * p_image;

        x[idx] = (1.0 - ZERO_GRADIENT_RELAXATION) * x_snapshot[idx] + ZERO_GRADIENT_RELAXATION * corrected_target;
    }
}

/// How many cells `laplacian_stencil4` (the residual check's operator) reads on each side of a
/// cell along each axis. Kept in sync with that function by hand, since it's a diagnostic-only
/// concern, not part of the solve itself.
pub(crate) const RESIDUAL_STENCIL_REACH_CELLS: usize = 2;

/// Extends `mask` (already `true` at every directly corrected interior cell) to also mark
/// every *uncorrected* cell whose own `laplacian_stencil4` residual stencil reads one of those
/// corrected cells — i.e. every cell within `RESIDUAL_STENCIL_REACH_CELLS` steps along a single
/// axis (matching the stencil's axis-aligned, non-diagonal reach) of a corrected cell. Shared by
/// `MultigridCPU`/`MultigridGPU`'s residual diagnostics.
pub(crate) fn dilate_exclusion_mask(grid: &Grid, mask: &mut [bool], cell_lookup: &[i32]) {
    let [nx, ny, nz] = grid.interior_shape;
    let [sx, sy, sz] = grid.interior_stride;

    for (cell_index, &lookup_index) in cell_lookup.iter().enumerate() {
        if lookup_index < 0 {
            continue;
        }

        let indices = grid.interior_indices_from_flat_index(cell_index);

        for (position, count, stride) in [(indices[0], nx, sx), (indices[1], ny, sy), (indices[2], nz, sz)] {
            for offset in 1..=RESIDUAL_STENCIL_REACH_CELLS {
                if position >= offset {
                    mask[cell_index - offset * stride] = true;
                }

                if position + offset < count {
                    mask[cell_index + offset * stride] = true;
                }
            }
        }
    }
}
