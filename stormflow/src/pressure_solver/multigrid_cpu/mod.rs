pub mod kernels;
pub mod settings;
pub mod slip_pressure_stencils;

use stormath::type_aliases::Float;
use stormath::matrix::Matrix;
use settings::{MultigridSettings, CoarsestLevelSolver};
use slip_pressure_stencils::SlipPressureStencils;

use kernels::{
    jacobi::{jacobi_kernel, jacobi_kernel_with_slip_correction, SLIP_CORRECTION_RELAXATION},
    restrict::compute_residual_and_restrict_kernel,
    prolongate::prolongate_and_correct_kernel,
    coarse_matrix::build_poisson_matrix4
};

use crate::{
    pressure_solver::boundary_conditions::PressureBoundaryConditions,
    geometry::Geometry,
    grid::Grid
};

pub struct MultigridCPU {
    pub grids: Vec<Grid>,
    pub boundary_conditions: PressureBoundaryConditions,
    pub solver_settings: MultigridSettings,
    /// Solution values at each grid level. Stored on the **interior** grid — there are no ghost
    /// cells anywhere in the solve; boundary conditions are folded directly into the stencils
    /// (see `kernels::off_diagonal_sum`).
    pub x_at_levels: Vec<Vec<Float>>,
    /// Work buffer for Jacobi iterations. Stored on the **interior** grid.
    pub x_at_levels_work: Vec<Vec<Float>>,
    /// Right-hand side at each grid level. Stored on the **interior** grid.
    pub rhs_at_levels: Vec<Vec<Float>>,
    /// Solution for the finest level, on the **extended** grid (matching `MultigridGPU::solution`,
    /// since `Simulation::update_velocity` needs the boundary-extrapolated pressure one cell past
    /// the domain edge). Materialized once at the end of `solve`, from `x_at_levels[0]` — see
    /// `solve` for details.
    pub solution: Vec<Float>,
    /// Dense matrix for the coarsest level's Poisson equation (same equation
    /// `poisson_jacobi_smoother` iterates towards, see `kernels::coarse_matrix`), built once here
    /// in `new` and reused for every V-cycle to solve that level exactly via Gaussian elimination
    /// instead of approximating it with extra Jacobi iterations.
    pub coarse_matrix: Matrix<Float>,
    /// Precomputed pressure zero-gradient (Neumann) correction entries near slip walls, one
    /// `SlipPressureStencils` per level of `grids` (empty if there are no slip geometries). Folded
    /// directly into `poisson_jacobi_smoother`'s per-cell Jacobi update (see
    /// `kernels::jacobi::jacobi_kernel_with_slip_correction`) when
    /// `solver_settings.enable_slip_pressure_correction` is set. Built once here in `new`, since
    /// the slip geometry is static.
    pub slip_pressure_stencils: Vec<SlipPressureStencils>,
}

impl MultigridCPU {
    pub fn new(
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
        solver_settings: MultigridSettings,
        slip_geometries: &[Geometry]
    ) -> Self {
        let grids = grid.multigrid_hierarchy();

        let mut x_at_levels: Vec<Vec<Float>> = Vec::new();
        let mut rhs_at_levels: Vec<Vec<Float>> = Vec::new();

        for level_grid in &grids {
            x_at_levels.push(
                vec![0.0; level_grid.nr_interior_cells()]
            );

            rhs_at_levels.push(
                vec![0.0; level_grid.nr_interior_cells()]
            );
        }

        let x_at_levels_work = x_at_levels.clone();
        let solution = vec![0.0; grids[0].nr_extended_cells()];

        let coarse_matrix = build_poisson_matrix4(
            grids.last().expect("grid hierarchy must have at least one level"),
            boundary_conditions
        );

        println!("Building per-level slip-pressure stencils");
        let slip_pressure_interpolation_order = solver_settings.slip_pressure_interpolation_order;
        let slip_pressure_stencils: Vec<SlipPressureStencils> = grids.iter()
            .map(|level_grid| SlipPressureStencils::build(level_grid, slip_geometries, slip_pressure_interpolation_order))
            .collect();

        Self {
            grids,
            boundary_conditions: boundary_conditions.clone(),
            solver_settings,
            x_at_levels,
            x_at_levels_work,
            rhs_at_levels,
            solution,
            coarse_matrix,
            slip_pressure_stencils,
        }
    }

    /// One-shot post-solve pressure correction for the coarsest level's exact (Gaussian
    /// elimination) solve. That solve isn't iterative, so there's nothing to fold the correction
    /// into the way `jacobi_kernel_with_slip_correction` does for the smoother — this reproduces
    /// the same blend directly, in place, sequentially (the coarsest level is small enough, by
    /// construction of the multigrid hierarchy, for that to be negligible).
    fn correct_coarsest_level_pressure_for_slip_geometry(&mut self) {
        let coarsest_level = self.grids.len() - 1;
        let stencils = &self.slip_pressure_stencils[coarsest_level];

        if stencils.entries.is_empty() {
            return;
        }

        let grid = &self.grids[coarsest_level];
        let stride = grid.interior_stride;
        let x_snapshot = self.x_at_levels[coarsest_level].clone();

        for (idx, &lookup_index) in stencils.cell_lookup.iter().enumerate() {
            if lookup_index < 0 {
                continue;
            }

            let entry = &stencils.entries[lookup_index as usize];
            let p_image = entry.stencil.sample_scalar(&x_snapshot, stride);
            let corrected_target = entry.mu * x_snapshot[idx] + (1.0 - entry.mu) * p_image;

            // Same under-relaxation as `jacobi_kernel_with_slip_correction`, for consistency.
            self.x_at_levels[coarsest_level][idx] = (1.0 - SLIP_CORRECTION_RELAXATION) * x_snapshot[idx]
                + SLIP_CORRECTION_RELAXATION * corrected_target;
        }
    }

    /// Computes residuals at the fine level and restricts them to the coarse level in a single fused pass.
    ///
    /// This avoids storing the full residual array at the fine level, reducing memory traffic.
    /// For each coarse cell, computes residuals for its 8 fine children on-the-fly and averages them.
    ///
    /// The residual for fine cell i is: r_i = rhs_i - (A * x)_i
    /// where A is the discrete Laplacian (Poisson matrix), with boundary conditions folded
    /// directly into the stencil for fine cells missing a real neighbor.
    ///
    /// # Grid layout
    /// - `x_fine`/`rhs_fine` are on the **interior** grid
    /// - `rhs_coarse` (output) is on the **interior** grid
    ///
    pub fn compute_residual_and_restrict(&mut self, fine_level: usize) {
        let coarse_level = fine_level + 1;

        let grid_fine = &self.grids[fine_level];
        let grid_coarse = &self.grids[coarse_level];
        let x_fine = &self.x_at_levels[fine_level];
        let boundary_conditions = &self.boundary_conditions;

        // `rhs_at_levels[fine_level]` (read) and `rhs_at_levels[coarse_level]` (written below)
        // alias the same Vec; split_at_mut borrows both disjointly without unsafe.
        let (rhs_below_coarse, rhs_from_coarse) = self.rhs_at_levels.split_at_mut(coarse_level);
        let rhs_fine = &rhs_below_coarse[fine_level];
        let rhs_coarse = &mut rhs_from_coarse[0];

        grid_coarse.parallel_interior_update(
            rhs_coarse,
            |_idx_coarse, indices_coarse, _current| {
                compute_residual_and_restrict_kernel(
                    indices_coarse,
                    grid_fine,
                    x_fine,
                    rhs_fine,
                    boundary_conditions
                )
            }
        );
    }

    /// Prolongates (interpolates) the correction from a coarser grid level to a finer level
    /// using trilinear interpolation, and adds it directly to x_at_levels[fine_level].
    ///
    /// This fuses the prolongation and correction addition into a single pass, avoiding
    /// the need for a separate corrections buffer and reducing memory traffic.
    ///
    /// Each fine cell value is updated as: x_fine += interpolated(x_coarse)
    ///
    /// # Grid layout
    /// - `x_fine`/`x_coarse` are on the **interior** grid
    ///
    pub fn prolongate_and_correct(&mut self, fine_level: usize) {
        let coarse_level = fine_level + 1;

        let fine_grid = &self.grids[fine_level];
        let coarse_grid = &self.grids[coarse_level];

        // `x_at_levels[fine_level]` (written below) and `x_at_levels[coarse_level]` (read) alias
        // the same Vec; split_at_mut borrows both disjointly without unsafe.
        let (x_below_coarse, x_from_coarse) = self.x_at_levels.split_at_mut(coarse_level);
        let x_fine = &mut x_below_coarse[fine_level];
        let coarse_values = &x_from_coarse[0];

        fine_grid.parallel_interior_update(
            x_fine,
            |_idx_fine, indices_fine, current| {
                let correction_value = prolongate_and_correct_kernel(
                    indices_fine,
                    coarse_grid,
                    coarse_values
                );

                current + correction_value
            }
        );
    }

    /// No ghost cells means no seeding step: every cell's new value only ever depends on
    /// `current`'s genuine interior values (with the boundary condition folded directly into the
    /// stencil for cells missing a real neighbor), never on a separately-maintained padding
    /// layer. So the very first iteration is always well-defined from whatever `current` already
    /// holds — after a clear, after prolongation, or carried over from a previous smoother call.
    ///
    /// When `solver_settings.enable_slip_pressure_correction` is set (and this level actually has
    /// any corrected cells), every sweep also folds in the slip-wall zero-gradient correction via
    /// `jacobi_kernel_with_slip_correction` instead of the plain `jacobi_kernel` — see that
    /// function's doc comment. Disabled (the default), this is byte-for-byte the same as before:
    /// no extra branching, no extra per-cell data touched.
    pub fn poisson_jacobi_smoother(&mut self, i_g: usize, nr_iterations: usize) {
        let grid = &self.grids[i_g];
        let boundary_conditions = &self.boundary_conditions;
        let rhs = &self.rhs_at_levels[i_g];
        let slip_pressure_stencils = &self.slip_pressure_stencils[i_g];

        let use_slip_correction = self.solver_settings.enable_slip_pressure_correction
            && !slip_pressure_stencils.entries.is_empty();

        for iteration in 0..nr_iterations {
            // Swap buffers: read from current, write to new
            // Even iterations: read from solution, write to work
            // Odd iterations: read from work, write to solution
            if iteration % 2 == 0 {
                let current = &self.x_at_levels[i_g];
                let out = &mut self.x_at_levels_work[i_g];

                jacobi_sweep(grid, boundary_conditions, rhs, current, out, slip_pressure_stencils, use_slip_correction);
            } else {
                let current = &self.x_at_levels_work[i_g];
                let out = &mut self.x_at_levels[i_g];

                jacobi_sweep(grid, boundary_conditions, rhs, current, out, slip_pressure_stencils, use_slip_correction);
            }
        }

        // If odd number of iterations, the result is in work; copy back to solution
        if nr_iterations % 2 == 1 {
            self.x_at_levels[i_g].copy_from_slice(&self.x_at_levels_work[i_g]);
        }
    }

    /// Solves the coarsest level's Poisson equation exactly via Gaussian elimination, using
    /// `coarse_matrix` (built once in `new` and reused across every V-cycle) instead of smoothing
    /// it approximately.
    pub fn solve_coarsest_level(&mut self) {
        let coarsest_level = self.grids.len() - 1;
        let rhs = &self.rhs_at_levels[coarsest_level];

        let solution = self.coarse_matrix.solve_gaussian_elimination(rhs)
            .expect("Coarsest multigrid level's Poisson matrix should be non-singular");

        self.x_at_levels[coarsest_level].copy_from_slice(&solution);
    }

    pub fn perform_v_cycle(&mut self) {
        let nr_grids = self.grids.len();

        let nr_iterations = self.solver_settings.nr_smooth_iterations;

        // Smooth and restrict down to the coarsest level
        for i_g in 0..nr_grids-1 {
            if i_g > 0 {
                self.x_at_levels[i_g].fill(0.0);
            }

            self.poisson_jacobi_smoother(i_g, nr_iterations);
            self.compute_residual_and_restrict(i_g);
        }

        match self.solver_settings.coarsest_level_solver {
            CoarsestLevelSolver::Exact => {
                self.solve_coarsest_level();

                // Not an iterative smoother, so there's no per-sweep pass to fold the correction
                // into — apply it once, directly, same as before this change.
                if self.solver_settings.enable_slip_pressure_correction {
                    self.correct_coarsest_level_pressure_for_slip_geometry();
                }
            },
            CoarsestLevelSolver::Jacobi => {
                self.x_at_levels[nr_grids - 1].fill(0.0);
                self.poisson_jacobi_smoother(nr_grids - 1, nr_iterations * 4);
            }
        }

        // Prolongate and smooth back up
        for i_g in (0..nr_grids-1).rev() {
            self.prolongate_and_correct(i_g);
            self.poisson_jacobi_smoother(i_g, nr_iterations);
        }
    }

    /// Solves the Poisson equation using multigrid V-cycles.
    ///
    /// # Note
    /// The caller is responsible for populating `rhs_at_levels[0]` (finest-level RHS on the
    /// interior grid) before calling this. On return, `self.solution` holds the finest-level
    /// solution on the extended grid.
    pub fn solve(&mut self) {
        for _ in 0..self.solver_settings.nr_v_cycles {
            self.perform_v_cycle();
        }

        // The solve never touches ghost cells (boundary conditions are folded into the
        // stencils), so materialize the extended layout `update_velocity`/`export_fields_as_vtk`
        // expect from the interior result.
        let x_interior = &self.x_at_levels[0];

        self.grids[0].parallel_interior_to_extended(
            &mut self.solution,
            |flat_interior| x_interior[flat_interior]
        );

        self.boundary_conditions.set_ghost_cells(&self.grids[0], &mut self.solution);

        if self.solver_settings.compute_residual_after_solve {
            // Slip-corrected cells don't satisfy Ax = rhs by construction (see
            // `kernels::compute_residual4`'s doc comment), so exclude them from the average —
            // otherwise the metric mixes genuine solve-quality with the boundary correction's
            // deliberate, permanent local mismatch. Also dilate the exclusion by
            // `RESIDUAL_STENCIL_REACH_CELLS`: any *uncorrected* cell whose own residual stencil
            // reads a corrected neighbor is checking `Ax = rhs` against data that includes a
            // deliberately non-solved value, so its residual is contaminated too even though it
            // was never itself patched.
            let mut excluded_slip_cells = vec![false; self.rhs_at_levels[0].len()];
            let mut nr_excluded = 0;

            if self.solver_settings.enable_slip_pressure_correction {
                let cell_lookup = &self.slip_pressure_stencils[0].cell_lookup;

                for (idx, &lookup_index) in cell_lookup.iter().enumerate() {
                    if lookup_index >= 0 {
                        excluded_slip_cells[idx] = true;
                    }
                }

                dilate_exclusion_mask(&self.grids[0], &mut excluded_slip_cells, cell_lookup);

                nr_excluded = excluded_slip_cells.iter().filter(|&&excluded| excluded).count();
            }

            let avg_residual = kernels::compute_residual4(
                &self.grids[0],
                &self.solution,
                &self.rhs_at_levels[0],
                &excluded_slip_cells
            );

            println!("Residual sum: {} ({} slip-boundary cells excluded)", avg_residual, nr_excluded);
        }
    }
}

/// How many cells `laplacian_stencil4` (the residual check's operator) reads on each side of a
/// cell along each axis. Kept in sync with that function by hand, since it's a diagnostic-only
/// concern, not part of the solve itself.
const RESIDUAL_STENCIL_REACH_CELLS: usize = 2;

/// Extends `mask` (already `true` at every directly slip-corrected interior cell) to also mark
/// every *uncorrected* cell whose own `laplacian_stencil4` residual stencil reads one of those
/// corrected cells — i.e. every cell within `RESIDUAL_STENCIL_REACH_CELLS` steps along a single
/// axis (matching the stencil's axis-aligned, non-diagonal reach) of a corrected cell.
fn dilate_exclusion_mask(grid: &Grid, mask: &mut [bool], cell_lookup: &[i32]) {
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

/// Runs one Jacobi sweep (`grid.parallel_interior_update`) from `current` into `out`, using
/// `jacobi_kernel_with_slip_correction` when `use_slip_correction` is set and plain `jacobi_kernel`
/// otherwise — chosen once per sweep, not per cell, so the disabled path never touches
/// `slip_pressure_stencils` at all.
#[inline(always)]
fn jacobi_sweep(
    grid: &Grid,
    boundary_conditions: &PressureBoundaryConditions,
    rhs: &[Float],
    current: &[Float],
    out: &mut [Float],
    slip_pressure_stencils: &SlipPressureStencils,
    use_slip_correction: bool
) {
    if use_slip_correction {
        grid.parallel_interior_update(
            out,
            |idx, indices, _current_out| {
                jacobi_kernel_with_slip_correction(
                    grid, boundary_conditions, rhs, current, idx, indices, slip_pressure_stencils
                )
            }
        );
    } else {
        grid.parallel_interior_update(
            out,
            |idx, indices, _current_out| {
                jacobi_kernel(grid, boundary_conditions, rhs, current, idx, indices)
            }
        );
    }
}
