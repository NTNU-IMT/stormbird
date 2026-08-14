
use stormath::type_aliases::Float;
use stormath::matrix::Matrix;

use super::{
    multigrid_cpu::settings::{MultigridSettings, CoarsestLevelSolver},
    boundary_conditions::PressureBoundaryConditions
};

use crate::grid::Grid;
use crate::geometry::Geometry;

pub mod kernels;

use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use kernels::jacobi_shader::{JacobiShader, WORKGROUP_SIZE as JACOBI_WORKGROUP_SIZE};
use kernels::jacobi_slip_shader::JacobiSlipShader;
use kernels::slip_pressure_gpu::gpu_buffers_from_stencils;
use kernels::restrict_shader::{RestrictShader, WORKGROUP_SIZE as RESTRICT_WORKGROUP_SIZE};
use kernels::prolongate_shader::{ProlongateShader, WORKGROUP_SIZE as PROLONGATE_WORKGROUP_SIZE};
use kernels::materialize_shader::MaterializeShader;

use crate::pressure_solver::multigrid_cpu::kernels as cpu_kernels;
use cpu_kernels::coarse_matrix::build_poisson_matrix4;
use crate::pressure_solver::multigrid_cpu::slip_pressure_stencils::{
    SlipPressureStencils, apply_relaxed_correction, dilate_exclusion_mask
};

/// All GPU resources belonging to a single multigrid level. `x_buffer`/`x_work_buffer`/
/// `rhs_buffer` are all sized to the **interior** grid — there are no ghost cells anywhere in the
/// solve; boundary conditions are folded directly into the stencils (see jacobi_shader.wgsl).
struct GpuLevel {
    grid_buffer: wgpu::Buffer,
    x_buffer: wgpu::Buffer,
    x_work_buffer: wgpu::Buffer,
    rhs_buffer: wgpu::Buffer,
    jacobi_bind_group_sol_to_work: wgpu::BindGroup,
    jacobi_bind_group_work_to_sol: wgpu::BindGroup,
    /// Workgroup dispatch counts for the Jacobi kernel on this level.
    jacobi_dispatch: [u32; 3],
    /// Slip-wall pressure correction buffers/bind groups for this level. `Some` only when
    /// `solver_settings.enable_slip_pressure_correction` is set *and* this level actually has
    /// corrected cells — mirrors `MultigridCPU::poisson_jacobi_smoother`'s per-level
    /// `use_slip_correction` gate. The 4 buffers are kept alive here even though nothing ever
    /// reads them back to the host or rewrites them after `MultigridGPU::new` — only the bind
    /// groups get used per V-cycle — matching this codebase's existing convention of keeping GPU
    /// buffers as owned fields rather than relying on bind groups to keep them alive implicitly.
    slip_buffers: Option<GpuSlipLevel>,
}

/// Per-level GPU resources for the slip-wall pressure correction — see `GpuLevel::slip_buffers`.
/// The 4 buffer fields are only ever read by the GPU (via the bind groups built from them in
/// `MultigridGPU::new`), never read back from Rust, hence `#[allow(dead_code)]`.
#[allow(dead_code)]
struct GpuSlipLevel {
    cell_lookup_buffer: wgpu::Buffer,
    weights_buffer: wgpu::Buffer,
    base_index_buffer: wgpu::Buffer,
    mu_buffer: wgpu::Buffer,
    bind_group_sol_to_work: wgpu::BindGroup,
    bind_group_work_to_sol: wgpu::BindGroup,
}

/// GPU resources needed to restrict a fine level's residual onto the next coarser level's RHS.
struct RestrictLevel {
    bind_group: wgpu::BindGroup,
    dispatch: [u32; 3],
}

/// GPU resources needed to prolongate a coarse level's solution onto the next finer level.
struct ProlongateLevel {
    bind_group: wgpu::BindGroup,
    dispatch: [u32; 3],
}

pub struct MultigridGPU {
    pub grids: Vec<Grid>,
    pub boundary_conditions: PressureBoundaryConditions,
    pub solver_settings: MultigridSettings,
    pub gpu_context: GpuContext,
    /// Right-hand side for the finest level, on the **interior** grid. Written by the caller
    /// (e.g. `Simulation::pressure_projection_rhs`) and uploaded to the GPU at the start of `solve`.
    pub rhs: Vec<Float>,
    /// Solution for the finest level, on the **extended** grid (matching `MultigridCPU::solution`,
    /// since `Simulation::update_velocity` needs the boundary-extrapolated pressure one cell past
    /// the domain edge). Materialized once at the end of `solve`, entirely on the GPU — see
    /// `MaterializeShader`.
    pub solution: Vec<Float>,

    jacobi_shader: JacobiShader,
    /// Only built when `solver_settings.enable_slip_pressure_correction` is set — compiling an
    /// unused pipeline has real GPU cost, unlike the CPU path's "always build the (cheap)
    /// stencils, gate only the dispatch."
    jacobi_slip_shader: Option<JacobiSlipShader>,
    restrict_shader: RestrictShader,
    prolongate_shader: ProlongateShader,
    materialize_shader: MaterializeShader,

    levels: Vec<GpuLevel>,
    restrict_levels: Vec<RestrictLevel>,
    prolongate_levels: Vec<ProlongateLevel>,

    /// Precomputed per-level slip-wall pressure correction stencils, built the same way
    /// `MultigridCPU`'s are (`SlipPressureStencils::build`). Kept around (not just consumed into
    /// GPU buffers) since `solve_coarsest_level_exact`'s host-side correction and `solve`'s
    /// residual-exclusion diagnostic both need host-side access to them.
    slip_pressure_stencils: Vec<SlipPressureStencils>,

    /// Dense matrix for the coarsest level's Poisson equation, built once here in `new` via the
    /// same `multigrid_cpu::kernels::coarse_matrix::build_poisson_matrix4` that `MultigridCPU`
    /// uses, and reused for every V-cycle when `solver_settings.coarsest_level_solver` is
    /// `CoarsestLevelSolver::Exact` (see `solve_coarsest_level_exact`).
    coarse_matrix: Matrix<Float>,
    /// Host-readable staging buffer for reading the coarsest level's restricted RHS back from the
    /// GPU when solving that level exactly on the CPU. Sized to the coarsest grid's interior cell
    /// count, matching `levels[coarsest_level].rhs_buffer`.
    coarse_rhs_staging_buffer: wgpu::Buffer,

    /// Bind group for `materialize_shader`, reading the finest level's converged `x_buffer` and
    /// writing into `solution_buffer`.
    materialize_bind_group: wgpu::BindGroup,
    /// Extended-sized GPU buffer that `materialize_shader` writes the final result into.
    solution_buffer: wgpu::Buffer,

    /// Sized to the finest level's **extended** cell count, matching `solution_buffer`.
    solution_staging_buffer: wgpu::Buffer,
}

impl MultigridGPU {
    pub fn new(
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
        solver_settings: MultigridSettings,
        slip_geometries: &[Geometry]
    ) -> Self {
        let grids = grid.multigrid_hierarchy();
        let nr_levels = grids.len();

        let gpu_context = GpuContext::new();

        let jacobi_shader = JacobiShader::new(&gpu_context, boundary_conditions);
        let restrict_shader = RestrictShader::new(&gpu_context, boundary_conditions);
        let prolongate_shader = ProlongateShader::new(&gpu_context);

        println!("Building per-level slip-pressure stencils");
        let slip_pressure_interpolation_order = solver_settings.slip_pressure_interpolation_order;
        let slip_pressure_stencils: Vec<SlipPressureStencils> = grids.iter()
            .map(|level_grid| SlipPressureStencils::build(level_grid, slip_geometries, slip_pressure_interpolation_order))
            .collect();

        let jacobi_slip_shader = if solver_settings.enable_slip_pressure_correction {
            Some(JacobiSlipShader::new(&gpu_context, boundary_conditions, slip_pressure_interpolation_order))
        } else {
            None
        };

        let mut levels: Vec<GpuLevel> = Vec::with_capacity(nr_levels);

        for (level_index, level_grid) in grids.iter().enumerate() {
            let grid_buffer = level_grid.as_gpu_version().as_buffer(&gpu_context);

            let interior_host = vec![0.0 as Float; level_grid.nr_interior_cells()];

            let x_buffer = gpu_context.create_buffer_from_src(&interior_host);
            let x_work_buffer = gpu_context.create_buffer_from_src(&interior_host);
            let rhs_buffer = gpu_context.create_buffer_from_src(&interior_host);

            let (jacobi_bind_group_sol_to_work, jacobi_bind_group_work_to_sol) = jacobi_shader.create_bind_groups(
                &gpu_context,
                &grid_buffer,
                &x_buffer,
                &rhs_buffer,
                &x_work_buffer
            );

            let jacobi_dispatch = [
                gpu_utils::workgroup_count(level_grid.interior_shape[0], JACOBI_WORKGROUP_SIZE as usize),
                gpu_utils::workgroup_count(level_grid.interior_shape[1], JACOBI_WORKGROUP_SIZE as usize),
                gpu_utils::workgroup_count(level_grid.interior_shape[2], JACOBI_WORKGROUP_SIZE as usize),
            ];

            let level_stencils = &slip_pressure_stencils[level_index];

            let slip_buffers = match &jacobi_slip_shader {
                Some(slip_shader) if !level_stencils.entries.is_empty() => {
                    let (weights, base_index, mu) = gpu_buffers_from_stencils(level_stencils);

                    let cell_lookup_buffer = gpu_context.create_storage_buffer_init(&level_stencils.cell_lookup);
                    let weights_buffer = gpu_context.create_storage_buffer_init(&weights);
                    let base_index_buffer = gpu_context.create_storage_buffer_init(&base_index);
                    let mu_buffer = gpu_context.create_storage_buffer_init(&mu);

                    let (bind_group_sol_to_work, bind_group_work_to_sol) = slip_shader.create_bind_groups(
                        &gpu_context,
                        &grid_buffer,
                        &x_buffer,
                        &rhs_buffer,
                        &x_work_buffer,
                        &cell_lookup_buffer,
                        &weights_buffer,
                        &base_index_buffer,
                        &mu_buffer,
                    );

                    Some(GpuSlipLevel {
                        cell_lookup_buffer,
                        weights_buffer,
                        base_index_buffer,
                        mu_buffer,
                        bind_group_sol_to_work,
                        bind_group_work_to_sol,
                    })
                },
                _ => None
            };

            levels.push(GpuLevel {
                grid_buffer,
                x_buffer,
                x_work_buffer,
                rhs_buffer,
                jacobi_bind_group_sol_to_work,
                jacobi_bind_group_work_to_sol,
                jacobi_dispatch,
                slip_buffers,
            });
        }

        let mut restrict_levels: Vec<RestrictLevel> = Vec::with_capacity(nr_levels.saturating_sub(1));
        let mut prolongate_levels: Vec<ProlongateLevel> = Vec::with_capacity(nr_levels.saturating_sub(1));

        for fine_level in 0..nr_levels.saturating_sub(1) {
            let coarse_level = fine_level + 1;

            let restrict_bind_group = restrict_shader.create_bind_group(
                &gpu_context,
                &levels[fine_level].grid_buffer,
                &levels[coarse_level].grid_buffer,
                &levels[fine_level].x_buffer,
                &levels[fine_level].rhs_buffer,
                &levels[coarse_level].rhs_buffer,
            );

            let coarse_grid = &grids[coarse_level];
            let restrict_dispatch = [
                gpu_utils::workgroup_count(coarse_grid.interior_shape[0], RESTRICT_WORKGROUP_SIZE as usize),
                gpu_utils::workgroup_count(coarse_grid.interior_shape[1], RESTRICT_WORKGROUP_SIZE as usize),
                gpu_utils::workgroup_count(coarse_grid.interior_shape[2], RESTRICT_WORKGROUP_SIZE as usize),
            ];

            restrict_levels.push(RestrictLevel {
                bind_group: restrict_bind_group,
                dispatch: restrict_dispatch,
            });

            let prolongate_bind_group = prolongate_shader.create_bind_group(
                &gpu_context,
                &levels[fine_level].grid_buffer,
                &levels[coarse_level].grid_buffer,
                &levels[coarse_level].x_buffer,
                &levels[fine_level].x_buffer,
            );

            let fine_grid = &grids[fine_level];
            let prolongate_dispatch = [
                gpu_utils::workgroup_count(fine_grid.interior_shape[0], PROLONGATE_WORKGROUP_SIZE as usize),
                gpu_utils::workgroup_count(fine_grid.interior_shape[1], PROLONGATE_WORKGROUP_SIZE as usize),
                gpu_utils::workgroup_count(fine_grid.interior_shape[2], PROLONGATE_WORKGROUP_SIZE as usize),
            ];

            prolongate_levels.push(ProlongateLevel {
                bind_group: prolongate_bind_group,
                dispatch: prolongate_dispatch,
            });
        }

        let coarsest_level = nr_levels - 1;
        let coarse_matrix = build_poisson_matrix4(&grids[coarsest_level], boundary_conditions);
        let coarse_rhs_staging_buffer = gpu_context.create_staging_buffer(grids[coarsest_level].nr_interior_cells());

        let materialize_shader = MaterializeShader::new(&gpu_context, boundary_conditions);
        let solution_host = vec![0.0 as Float; grids[0].nr_extended_cells()];
        let solution_buffer = gpu_context.create_buffer_from_src(&solution_host);
        let materialize_bind_group = materialize_shader.create_bind_group(
            &gpu_context,
            &levels[0].grid_buffer,
            &levels[0].x_buffer,
            &solution_buffer
        );

        let solution = vec![0.0 as Float; grids[0].nr_extended_cells()];
        let rhs = vec![0.0 as Float; grids[0].nr_interior_cells()];
        let solution_staging_buffer = gpu_context.create_staging_buffer(grids[0].nr_extended_cells());

        Self {
            grids,
            boundary_conditions: boundary_conditions.clone(),
            solver_settings,
            gpu_context,
            rhs,
            solution,
            jacobi_shader,
            jacobi_slip_shader,
            restrict_shader,
            prolongate_shader,
            materialize_shader,
            levels,
            restrict_levels,
            prolongate_levels,
            slip_pressure_stencils,
            coarse_matrix,
            coarse_rhs_staging_buffer,
            materialize_bind_group,
            solution_buffer,
            solution_staging_buffer,
        }
    }

    /// No ghost cells means no seeding step: every cell's new value only ever depends on
    /// `current`'s genuine interior values (with the boundary condition folded directly into the
    /// stencil for cells missing a real neighbor), never on a separately-maintained padding
    /// layer. So the very first iteration is always well-defined from whatever `current` already
    /// holds — after a clear, after prolongation, or carried over from a previous smoother call.
    ///
    /// Picks the plain (`jacobi_shader`) or slip-correction (`jacobi_slip_shader`) pipeline once
    /// per call — not per dispatched invocation — based on `level_data.slip_buffers`, the direct
    /// GPU analogue of `MultigridCPU::poisson_jacobi_smoother`'s `use_slip_correction` gate.
    fn poisson_jacobi_smoother_gpu(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        level: usize,
        nr_iterations: usize
    ) {
        let level_data = &self.levels[level];

        let pipeline = match &level_data.slip_buffers {
            Some(_) => &self.jacobi_slip_shader.as_ref()
                .expect("level_data.slip_buffers is only Some when jacobi_slip_shader was built")
                .pipeline,
            None => &self.jacobi_shader.pipeline,
        };

        for iteration in 0..nr_iterations {
            let bind_group = match (&level_data.slip_buffers, iteration % 2 == 0) {
                (Some(slip_level), true) => &slip_level.bind_group_sol_to_work,
                (Some(slip_level), false) => &slip_level.bind_group_work_to_sol,
                (None, true) => &level_data.jacobi_bind_group_sol_to_work,
                (None, false) => &level_data.jacobi_bind_group_work_to_sol,
            };

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(
                level_data.jacobi_dispatch[0],
                level_data.jacobi_dispatch[1],
                level_data.jacobi_dispatch[2]
            );
        }

        // If odd number of iterations, the result is in the work buffer; copy back to solution.
        if nr_iterations % 2 == 1 {
            let byte_len = GpuContext::byte_length_from_length(self.grids[level].nr_interior_cells());
            encoder.copy_buffer_to_buffer(&level_data.x_work_buffer, 0, &level_data.x_buffer, 0, byte_len);
        }
    }

    fn compute_residual_and_restrict_gpu(&self, encoder: &mut wgpu::CommandEncoder, fine_level: usize) {
        let restrict_level = &self.restrict_levels[fine_level];

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.restrict_shader.pipeline);
        pass.set_bind_group(0, &restrict_level.bind_group, &[]);
        pass.dispatch_workgroups(restrict_level.dispatch[0], restrict_level.dispatch[1], restrict_level.dispatch[2]);
    }

    fn prolongate_and_correct_gpu(&self, encoder: &mut wgpu::CommandEncoder, fine_level: usize) {
        let prolongate_level = &self.prolongate_levels[fine_level];

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.prolongate_shader.pipeline);
        pass.set_bind_group(0, &prolongate_level.bind_group, &[]);
        pass.dispatch_workgroups(prolongate_level.dispatch[0], prolongate_level.dispatch[1], prolongate_level.dispatch[2]);
    }

    fn clear_level_solution(&self, encoder: &mut wgpu::CommandEncoder, level: usize) {
        encoder.clear_buffer(&self.levels[level].x_buffer, 0, None);
    }

    /// Solves the coarsest level's Poisson equation exactly on the CPU, reusing
    /// `multigrid_cpu::kernels::coarse_matrix::build_poisson_matrix4` and
    /// `Matrix::solve_gaussian_elimination` — the same machinery `MultigridCPU` uses for its own
    /// coarsest level (see `MultigridCPU::coarse_matrix`/`solve_coarsest_level`) — instead of
    /// duplicating a GPU-side direct solver.
    ///
    /// `encoder` must already contain the down-sweep through the coarsest level's restricted RHS.
    /// This submits it, blocks on reading that RHS back to the host, solves on the CPU, and writes
    /// the solution back into the coarsest level's `x_buffer`, returning a fresh encoder for the
    /// caller to record the up-sweep into. This host round-trip is the unavoidable cost of an
    /// exact coarsest-level solve; `CoarsestLevelSolver::Jacobi` avoids it by staying on the GPU.
    ///
    /// Since this already has the coarsest level's solution on the host, the slip-wall correction
    /// (when enabled) is applied right here in plain Rust via the same
    /// `apply_relaxed_correction` `MultigridCPU`'s equivalent coarsest-level method uses — no GPU
    /// kernel needed for this one-shot, non-iterative branch.
    fn solve_coarsest_level_exact(&self, mut encoder: wgpu::CommandEncoder) -> wgpu::CommandEncoder {
        let coarsest_level = self.levels.len() - 1;

        let byte_len = GpuContext::byte_length_from_length(self.grids[coarsest_level].nr_interior_cells());
        encoder.copy_buffer_to_buffer(
            &self.levels[coarsest_level].rhs_buffer, 0,
            &self.coarse_rhs_staging_buffer, 0,
            byte_len
        );

        let submission_index = self.gpu_context.queue.submit([encoder.finish()]);
        let rhs_host = self.gpu_context.read_from_staging_buffer(&self.coarse_rhs_staging_buffer, submission_index);

        let mut x_host = self.coarse_matrix.solve_gaussian_elimination(&rhs_host)
            .expect("Coarsest multigrid level's Poisson matrix should be non-singular");

        if self.solver_settings.enable_slip_pressure_correction {
            let stride = self.grids[coarsest_level].interior_stride;
            apply_relaxed_correction(&mut x_host, &self.slip_pressure_stencils[coarsest_level], stride);
        }

        self.gpu_context.write_buffer(&self.levels[coarsest_level].x_buffer, &x_host);

        self.gpu_context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
    }

    /// Records (and, when the coarsest level is solved exactly, submits) one V-cycle. Takes
    /// ownership of `encoder` rather than borrowing it, since `CoarsestLevelSolver::Exact` needs
    /// to submit/finish the down-sweep's encoder and hand back a new one for the up-sweep — see
    /// `solve_coarsest_level_exact`.
    fn perform_v_cycle(&self, mut encoder: wgpu::CommandEncoder) -> wgpu::CommandEncoder {
        let nr_levels = self.levels.len();
        let nr_iterations = self.solver_settings.nr_smooth_iterations;

        // Smooth and restrict down to the coarsest level.
        for level in 0..nr_levels - 1 {
            if level > 0 {
                self.clear_level_solution(&mut encoder, level);
            }

            self.poisson_jacobi_smoother_gpu(&mut encoder, level, nr_iterations);
            self.compute_residual_and_restrict_gpu(&mut encoder, level);
        }

        self.clear_level_solution(&mut encoder, nr_levels - 1);

        encoder = match self.solver_settings.coarsest_level_solver {
            CoarsestLevelSolver::Exact => self.solve_coarsest_level_exact(encoder),
            CoarsestLevelSolver::Jacobi => {
                self.poisson_jacobi_smoother_gpu(&mut encoder, nr_levels - 1, nr_iterations*4);
                encoder
            }
        };

        // Prolongate and smooth back up.
        for level in (0..nr_levels - 1).rev() {
            self.prolongate_and_correct_gpu(&mut encoder, level);
            self.poisson_jacobi_smoother_gpu(&mut encoder, level, nr_iterations);
        }

        encoder
    }

    /// Solves the Poisson equation using multigrid V-cycles on the GPU.
    ///
    /// # Note
    /// The caller is responsible for populating `self.rhs` (finest-level RHS on the interior
    /// grid) before calling this. On return, `self.solution` holds the finest-level solution on
    /// the extended grid, matching `MultigridCPU::solution`'s layout.
    pub fn solve(&mut self) {
        self.gpu_context.write_buffer(&self.levels[0].rhs_buffer, &self.rhs);

        let mut encoder = self.gpu_context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default()
        );

        for _ in 0..self.solver_settings.nr_v_cycles {
            encoder = self.perform_v_cycle(encoder);
        }

        // Materialize the extended layout `update_velocity`/`export_fields_as_vtk` expect
        // (interior values plus boundary-extrapolated ghost values) as one more dispatch in
        // whichever encoder the last V-cycle handed back, instead of a separate CPU pass that
        // could only start after the GPU work and its readback had already finished. With
        // `CoarsestLevelSolver::Jacobi` that's the same single command buffer spanning the whole
        // solve; with `CoarsestLevelSolver::Exact` it's just the up-sweep of the final V-cycle,
        // since each V-cycle's coarsest-level readback already forced its own submission.
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.materialize_shader.pipeline);
            pass.set_bind_group(0, &self.materialize_bind_group, &[]);
            pass.dispatch_workgroups(
                self.levels[0].jacobi_dispatch[0],
                self.levels[0].jacobi_dispatch[1],
                self.levels[0].jacobi_dispatch[2]
            );
        }

        let byte_len = GpuContext::byte_length_from_length(self.grids[0].nr_extended_cells());
        encoder.copy_buffer_to_buffer(&self.solution_buffer, 0, &self.solution_staging_buffer, 0, byte_len);

        let submission_index = self.gpu_context.queue.submit([encoder.finish()]);

        self.solution = self.gpu_context.read_from_staging_buffer(&self.solution_staging_buffer, submission_index);

        if self.solver_settings.compute_residual_after_solve {
            // Same widened exclusion mask as `MultigridCPU::solve` — see
            // `kernels::compute_residual4`'s doc comment for why slip-corrected cells (and their
            // immediate residual-stencil neighbors) need to be excluded from the average.
            let mut excluded_slip_cells = vec![false; self.rhs.len()];
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

            let avg_residual = cpu_kernels::compute_residual4(
                &self.grids[0], &self.solution, &self.rhs, &excluded_slip_cells
            );

            println!("Residual sum: {} ({} slip-boundary cells excluded)", avg_residual, nr_excluded);
        }
    }
}
