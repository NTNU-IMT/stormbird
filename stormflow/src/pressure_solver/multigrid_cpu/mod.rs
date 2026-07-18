pub mod kernels;
pub mod settings;

use stormath::type_aliases::Float;
use settings::MultigridSettings;



use kernels::{
    jacobi::jacobi_iteration_step,
    restrict::compute_residual_and_restrict_kernel,
    prolongate::prolongate_and_correct_kernel
};

use rayon::prelude::*;

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

        Self {
            grids,
            boundary_conditions: boundary_conditions.clone(),
            solver_settings,
            x_at_levels,
            x_at_levels_work,
            rhs_at_levels,
            solution,
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
    /// # Safety
    /// Uses unsafe pointer access to enable parallel writes. This is safe because each
    /// coarse cell index is processed exactly once, so there are no data races.
    pub fn compute_residual_and_restrict(&mut self, fine_level: usize) {
        let coarse_level = fine_level + 1;

        let grid_fine = &self.grids[fine_level];
        let grid_coarse = &self.grids[coarse_level];

        let nr_coarse_interior_cells = grid_coarse.nr_interior_cells();

        let rhs_coarse_ptr = self.rhs_at_levels[coarse_level].as_mut_ptr() as usize;
        let x_fine = &self.x_at_levels[fine_level];
        let rhs_fine = &self.rhs_at_levels[fine_level];
        let boundary_conditions = &self.boundary_conditions;

        (0..nr_coarse_interior_cells)
            .into_par_iter()
            .for_each(|flat_index_coarse_interior| {
                let restricted_value = compute_residual_and_restrict_kernel(
                    flat_index_coarse_interior,
                    grid_fine,
                    grid_coarse,
                    x_fine,
                    rhs_fine,
                    boundary_conditions
                );

                // Write result using unsafe pointer access
                // Safety: Each flat_index_coarse_interior is unique, so no data races occur
                unsafe {
                    let ptr = rhs_coarse_ptr as *mut Float;
                    *ptr.add(flat_index_coarse_interior) = restricted_value;
                }
            });
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
    /// # Safety
    /// Uses unsafe pointer access to enable parallel read-modify-write. This is safe because
    /// each fine cell index is processed exactly once, so there are no data races.
    pub fn prolongate_and_correct(&mut self, fine_level: usize) {
        let coarse_level = fine_level + 1;

        let fine_grid = &self.grids[fine_level];
        let coarse_grid = &self.grids[coarse_level];

        let [nx_f, ny_f, nz_f] = fine_grid.interior_shape;

        // Get raw pointers for parallel access
        let x_fine_ptr = self.x_at_levels[fine_level].as_mut_ptr() as usize;
        let coarse_values = &self.x_at_levels[coarse_level];

        (0..nx_f).into_par_iter().for_each(|i_f| {
            for j_f in 0..ny_f {
                for k_f in 0..nz_f {
                    let idx_fine = fine_grid.flat_index_on_interior_grid([i_f, j_f, k_f]);

                    let correction_value = prolongate_and_correct_kernel(
                        [i_f, j_f, k_f],
                        coarse_grid,
                        coarse_values
                    );

                    unsafe {
                        let ptr = x_fine_ptr as *mut Float;
                        *ptr.add(idx_fine) += correction_value;
                    }
                }
            }
        });
    }

    /// No ghost cells means no seeding step: every cell's new value only ever depends on
    /// `current`'s genuine interior values (with the boundary condition folded directly into the
    /// stencil for cells missing a real neighbor), never on a separately-maintained padding
    /// layer. So the very first iteration is always well-defined from whatever `current` already
    /// holds — after a clear, after prolongation, or carried over from a previous smoother call.
    pub fn poisson_jacobi_smoother(&mut self, i_g: usize, nr_iterations: usize) {
        for iteration in 0..nr_iterations {
            // Swap buffers: read from current, write to new
            // Even iterations: read from solution, write to work
            // Odd iterations: read from work, write to solution
            if iteration % 2 == 0 {
                jacobi_iteration_step(
                    &self.grids[i_g],
                    &self.boundary_conditions,
                    &self.rhs_at_levels[i_g],
                    &self.x_at_levels[i_g],
                    &mut self.x_at_levels_work[i_g]
                );
            } else {
                jacobi_iteration_step(
                    &self.grids[i_g],
                    &self.boundary_conditions,
                    &self.rhs_at_levels[i_g],
                    &self.x_at_levels_work[i_g],
                    &mut self.x_at_levels[i_g]
                );
            }
        }

        // If odd number of iterations, the result is in work; copy back to solution
        if nr_iterations % 2 == 1 {
            self.x_at_levels[i_g].copy_from_slice(&self.x_at_levels_work[i_g]);
        }
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

        self.x_at_levels[nr_grids - 1].fill(0.0);

        self.poisson_jacobi_smoother(nr_grids-1, nr_iterations * 4);

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
        let grid = &self.grids[0];
        let x_interior = &self.x_at_levels[0];
        let solution_ptr = self.solution.as_mut_ptr() as usize;

        (0..x_interior.len())
            .into_par_iter()
            .for_each(|flat_interior| {
                let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
                let flat_extended = grid.flat_index_on_extended_grid_from_interior_indices(interior_indices);

                // Safety: flat_extended is unique per flat_interior (bijective interior <->
                // extended mapping), so each thread writes a distinct index.
                unsafe {
                    let ptr = solution_ptr as *mut Float;
                    *ptr.add(flat_extended) = x_interior[flat_interior];
                }
            });

        self.boundary_conditions.set_ghost_cells(&self.grids[0], &mut self.solution);

        if self.solver_settings.compute_residual_after_solve {
            // Compute residual using stencil-based approach
            let avg_residual = kernels::compute_residual(
                &self.grids[0],
                &self.solution,
                &self.rhs_at_levels[0]
            );
    
            println!("Residual sum: {}", avg_residual);
        }
    }
}
