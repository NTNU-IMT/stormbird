pub mod kernels;
pub mod settings;

use stormath::type_aliases::Float;
use stormath::matrix::Matrix;
use settings::{MultigridSettings, CoarsestLevelSolver};

use kernels::{
    jacobi::jacobi_kernel,
    restrict::compute_residual_and_restrict_kernel,
    prolongate::prolongate_and_correct_kernel,
    coarse_matrix::build_poisson_matrix4
};

use crate::{
    pressure_solver::boundary_conditions::PressureBoundaryConditions,
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
}

impl MultigridCPU {
    pub fn new(
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
        solver_settings: MultigridSettings
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

        Self {
            grids,
            boundary_conditions: boundary_conditions.clone(),
            solver_settings,
            x_at_levels,
            x_at_levels_work,
            rhs_at_levels,
            solution,
            coarse_matrix,
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
    pub fn poisson_jacobi_smoother(&mut self, i_g: usize, nr_iterations: usize) {
        let grid = &self.grids[i_g];
        let boundary_conditions = &self.boundary_conditions;
        let rhs = &self.rhs_at_levels[i_g];

        for iteration in 0..nr_iterations {
            // Swap buffers: read from current, write to new
            // Even iterations: read from solution, write to work
            // Odd iterations: read from work, write to solution
            if iteration % 2 == 0 {
                let current = &self.x_at_levels[i_g];

                grid.parallel_interior_update(
                    &mut self.x_at_levels_work[i_g],
                    |idx, indices, _current_out| {
                        jacobi_kernel(grid, boundary_conditions, rhs, current, idx, indices)
                    }
                );
            } else {
                let current = &self.x_at_levels_work[i_g];

                grid.parallel_interior_update(
                    &mut self.x_at_levels[i_g],
                    |idx, indices, _current_out| {
                        jacobi_kernel(grid, boundary_conditions, rhs, current, idx, indices)
                    }
                );
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
            CoarsestLevelSolver::Exact => self.solve_coarsest_level(),
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
            // Compute residual using stencil-based approach
            let avg_residual = kernels::compute_residual4(
                &self.grids[0],
                &self.solution,
                &self.rhs_at_levels[0]
            );
    
            println!("Residual sum: {}", avg_residual);
        }
    }
}
