use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;
use crate::grid::interpolation::{TrilinearStencil, TricubicStencil};
use crate::geometry::{Geometry, WallGeometries};

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

/// Safety margin, as a multiple of the largest cell length of the finer level, added to the
/// distance between a coarse cell center and its children when bounding the signed distance
/// function on a coarse level (see `SignedDistanceBounds::coarsened`). Covers round-off errors in
/// the signed distance function, so that the pruning never skips a cell that would have gotten an
/// entry if the signed distance function had been evaluated directly.
const SIGNED_DISTANCE_BOUND_MARGIN_CELLS: Float = 0.1;

fn max_cell_length(grid: &Grid) -> Float {
    let mut max_dx = 0.0;
    for axis_index in 0..3 {
        if grid.cell_length[axis_index] > max_dx {
            max_dx = grid.cell_length[axis_index];
        }
    }

    max_dx
}

/// Lower and upper bounds on the signed distance function of the wall geometries, for each
/// interior cell (flat index) of one multigrid level. Evaluating the signed distance function is
/// the expensive part of building the stencils, so on the coarse levels it is only evaluated
/// exactly (giving `lower == upper`) for the cells that the bounds from the finer level can't rule
/// out from getting an entry. Since only cells with `-reach_distance < sdf < 0` get an entry, this
/// is a thin band of cells around the wall surfaces.
struct SignedDistanceBounds {
    lower: Vec<Float>,
    upper: Vec<Float>,
}

impl SignedDistanceBounds {
    /// Exact values on the finest level, taken from the precomputed signed distance functions of
    /// the selected walls. The union of several sets of walls is the minimum of their signed
    /// distance functions.
    fn finest_level(grid: &Grid, selected_walls: &[&WallGeometries]) -> Self {
        let sdf: Vec<Float> = (0..grid.nr_interior_cells()).into_par_iter()
            .map(|cell_index| {
                let interior_indices = grid.interior_indices_from_flat_index(cell_index);
                let i_extended = grid.flat_index_on_extended_grid_from_interior_indices(interior_indices);

                selected_walls.iter()
                    .map(|walls| walls.signed_distance_function[i_extended])
                    .fold(Float::MAX, Float::min)
            }).collect();

        Self {
            lower: sdf.clone(),
            upper: sdf,
        }
    }

    /// Bounds on `coarse_grid`, derived from the bounds on the next finer level. The signed
    /// distance function changes by at most the distance moved, and each coarse cell center is at
    /// a distance of half a fine cell diagonal from the centers of its eight children. Coarse
    /// cells where the resulting bounds show that they can't get an entry (see `build_level`) keep
    /// the bounds, while the signed distance function is evaluated exactly for the rest.
    fn coarsened(
        &self,
        fine_grid: &Grid,
        coarse_grid: &Grid,
        wall_geometries: &[Geometry]
    ) -> Self {
        let child_distance = 0.5 * fine_grid.cell_length.length() +
            SIGNED_DISTANCE_BOUND_MARGIN_CELLS * max_cell_length(fine_grid);

        let reach_distance = ZERO_GRADIENT_REACH_CELLS * max_cell_length(coarse_grid);

        let (lower, upper) = (0..coarse_grid.nr_interior_cells()).into_par_iter()
            .map(|coarse_index| {
                let [ic, jc, kc] = coarse_grid.interior_indices_from_flat_index(coarse_index);

                let mut max_fine_lower = -Float::MAX;
                let mut min_fine_upper = Float::MAX;

                for i_offset in 0..2 {
                    for j_offset in 0..2 {
                        for k_offset in 0..2 {
                            let fine_index = fine_grid.flat_index_on_interior_grid(
                                [2 * ic + i_offset, 2 * jc + j_offset, 2 * kc + k_offset]
                            );

                            max_fine_lower = max_fine_lower.max(self.lower[fine_index]);
                            min_fine_upper = min_fine_upper.min(self.upper[fine_index]);
                        }
                    }
                }

                let lower = max_fine_lower - child_distance;
                let upper = min_fine_upper + child_distance;

                if lower >= 0.0 || upper <= -reach_distance {
                    (lower, upper)
                } else {
                    let extended_indices = coarse_grid.extended_indices_from_interior_indices([ic, jc, kc]);

                    let sdf = Geometry::signed_distance_function_union(
                        wall_geometries,
                        coarse_grid.cell_center_extended(extended_indices)
                    );

                    (sdf, sdf)
                }
            }).unzip();

        Self { lower, upper }
    }
}

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
    /// Builds the stencils for every level in `grids` (a multigrid hierarchy, where the first level
    /// must be the grid that the signed distance functions in `slip_walls`/`no_slip_walls` are
    /// computed on), for the walls selected by `settings.zero_gradient_on_walls`. All levels get
    /// empty stencils if the condition is not used. Shared by `MultigridCPU` and `MultigridGPU`.
    pub fn build_for_all_levels(
        grids: &[Grid],
        settings: &MultigridSettings,
        slip_walls: &WallGeometries,
        no_slip_walls: &WallGeometries
    ) -> Vec<Self> {
        let selected_walls = settings.zero_gradient_on_walls.selected_walls(
            slip_walls, no_slip_walls
        );

        if selected_walls.is_empty() {
            return vec![Self::default(); grids.len()];
        }

        println!("Building per-level zero-gradient stencils");

        let wall_geometries: Vec<Geometry> = selected_walls.iter()
            .flat_map(|walls| walls.geometries.iter().cloned())
            .collect();

        let mut stencils = Vec::with_capacity(grids.len());
        let mut bounds = SignedDistanceBounds::finest_level(&grids[0], &selected_walls);

        for (level, level_grid) in grids.iter().enumerate() {
            if level > 0 {
                bounds = bounds.coarsened(&grids[level - 1], level_grid, &wall_geometries);
            }

            stencils.push(
                Self::build_level(level_grid, &wall_geometries, &bounds, settings.zero_gradient_interpolation_order)
            );
        }

        stencils
    }

    /// Builds the pressure zero-gradient correction entries for `grid` (one level of a multigrid
    /// hierarchy), `wall_geometries`, and the chosen interpolation `order`. The signed distance
    /// function bounds and the normals are only needed transiently here to build the stencils, not
    /// kept around afterward — everything needed at solve time ends up baked into the returned
    /// `entries`/`cell_lookup`. The normals are only computed for the cells that get an entry.
    fn build_level(
        grid: &Grid,
        wall_geometries: &[Geometry],
        bounds: &SignedDistanceBounds,
        order: ZeroGradientInterpolationOrder
    ) -> Self {
        // The reach also doubles as the blending width, so `mu` saturates to 0 exactly at the
        // pruning boundary (see `ZERO_GRADIENT_REACH_CELLS`'s doc comment).
        let epsilon = ZERO_GRADIENT_REACH_CELLS * max_cell_length(grid);
        let reach_distance = epsilon;

        let normal_delta = 0.1 * grid.cell_length;

        let field_origin = grid.cell_center([0, 0, 0]);

        let corrected_cells: Vec<(usize, ZeroGradientEntry)> = (0..grid.nr_interior_cells()).into_par_iter()
            .filter_map(|cell_index| {
                // Fluid-side cells (sdf >= 0) must stay untouched, and cells deeper than
                // `reach_distance` can never affect the pressure field outside the body at
                // this level's resolution — skip both by simply not creating an entry. This also
                // skips all cells where the signed distance function is only bounded.
                if bounds.lower[cell_index] >= 0.0 || bounds.upper[cell_index] <= -reach_distance {
                    return None;
                }

                debug_assert!(bounds.lower[cell_index] == bounds.upper[cell_index]);

                let sdf = bounds.lower[cell_index];

                let mu = Geometry::blending_function(sdf, epsilon);

                let interior_indices = grid.interior_indices_from_flat_index(cell_index);
                let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);

                let cell_center = grid.cell_center_extended(extended_indices);

                // Reflect the cell center across the (locally linear) interface to get the
                // image point on the fluid side
                let (_normal, image_point) = Geometry::mirror_image_point(
                    wall_geometries, cell_center, sdf, normal_delta
                );

                let stencil = match order {
                    ZeroGradientInterpolationOrder::Trilinear => ZeroGradientInterpolationStencil::Trilinear(
                        grid.trilinear_stencil_at_interior(field_origin, image_point)
                    ),
                    ZeroGradientInterpolationOrder::Tricubic => ZeroGradientInterpolationStencil::Tricubic(
                        grid.tricubic_stencil_at_interior(field_origin, image_point)
                    ),
                };

                Some((cell_index, ZeroGradientEntry { mu, stencil }))
            }).collect();

        let mut entries = Vec::with_capacity(corrected_cells.len());
        let mut cell_lookup = vec![-1i32; grid.nr_interior_cells()];

        for (cell_index, entry) in corrected_cells {
            cell_lookup[cell_index] = entries.len() as i32;
            entries.push(entry);
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
