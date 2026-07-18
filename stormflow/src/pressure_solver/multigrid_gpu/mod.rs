
use stormath::type_aliases::Float;

use super::{
    multigrid_cpu::settings::MultigridSettings,
    boundary_conditions::PressureBoundaryConditions
};

use crate::grid::Grid;

pub mod kernels;

use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use kernels::jacobi_shader::{JacobiShader, WORKGROUP_SIZE as JACOBI_WORKGROUP_SIZE};
use kernels::restrict_shader::{RestrictShader, WORKGROUP_SIZE as RESTRICT_WORKGROUP_SIZE};
use kernels::prolongate_shader::{ProlongateShader, WORKGROUP_SIZE as PROLONGATE_WORKGROUP_SIZE};
use kernels::coarse_solve_shader::CoarseSolveShader;
use kernels::materialize_shader::MaterializeShader;

use crate::pressure_solver::multigrid_cpu::kernels as cpu_kernels;

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
    restrict_shader: RestrictShader,
    prolongate_shader: ProlongateShader,
    materialize_shader: MaterializeShader,

    levels: Vec<GpuLevel>,
    restrict_levels: Vec<RestrictLevel>,
    prolongate_levels: Vec<ProlongateLevel>,

    /// Runs the entire bottom-of-V-cycle smoother for the coarsest level in a single dispatch.
    /// `None` if the coarsest grid doesn't fit in one workgroup on this device, in which case the
    /// coarsest level falls back to the regular per-iteration `poisson_jacobi_smoother_gpu` path.
    coarse_solve: Option<CoarseSolveShader>,

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
        solver_settings: MultigridSettings
    ) -> Self {
        let grids = grid.multigrid_hierarchy();
        let nr_levels = grids.len();

        let gpu_context = GpuContext::new();

        let jacobi_shader = JacobiShader::new(&gpu_context, boundary_conditions);
        let restrict_shader = RestrictShader::new(&gpu_context, boundary_conditions);
        let prolongate_shader = ProlongateShader::new(&gpu_context);

        let mut levels: Vec<GpuLevel> = Vec::with_capacity(nr_levels);

        for level_grid in &grids {
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

            levels.push(GpuLevel {
                grid_buffer,
                x_buffer,
                x_work_buffer,
                rhs_buffer,
                jacobi_bind_group_sol_to_work,
                jacobi_bind_group_work_to_sol,
                jacobi_dispatch,
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
        let coarse_solve = CoarseSolveShader::try_new(
            &gpu_context,
            &grids[coarsest_level],
            boundary_conditions,
            solver_settings.nr_smooth_iterations * 4,
            &levels[coarsest_level].grid_buffer,
            &levels[coarsest_level].rhs_buffer,
            &levels[coarsest_level].x_buffer,
        );

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
            restrict_shader,
            prolongate_shader,
            materialize_shader,
            levels,
            restrict_levels,
            prolongate_levels,
            coarse_solve,
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
    fn poisson_jacobi_smoother_gpu(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        level: usize,
        nr_iterations: usize
    ) {
        let level_data = &self.levels[level];

        for iteration in 0..nr_iterations {
            let bind_group = if iteration % 2 == 0 {
                &level_data.jacobi_bind_group_sol_to_work
            } else {
                &level_data.jacobi_bind_group_work_to_sol
            };

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.jacobi_shader.pipeline);
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

    fn perform_v_cycle(&self, encoder: &mut wgpu::CommandEncoder) {
        let nr_levels = self.levels.len();
        let nr_iterations = self.solver_settings.nr_smooth_iterations;

        // Smooth and restrict down to the coarsest level.
        for level in 0..nr_levels - 1 {
            if level > 0 {
                self.clear_level_solution(encoder, level);
            }

            self.poisson_jacobi_smoother_gpu(encoder, level, nr_iterations);
            self.compute_residual_and_restrict_gpu(encoder, level);
        }

        self.clear_level_solution(encoder, nr_levels - 1);

        if let Some(coarse_solve) = &self.coarse_solve {
            coarse_solve.dispatch(encoder);
        } else {
            self.poisson_jacobi_smoother_gpu(encoder, nr_levels - 1, nr_iterations * 4);
        }

        // Prolongate and smooth back up.
        for level in (0..nr_levels - 1).rev() {
            self.prolongate_and_correct_gpu(encoder, level);
            self.poisson_jacobi_smoother_gpu(encoder, level, nr_iterations);
        }
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
            self.perform_v_cycle(&mut encoder);
        }

        // Materialize the extended layout `update_velocity`/`export_fields_as_vtk` expect
        // (interior values plus boundary-extrapolated ghost values) as one more dispatch in the
        // same command buffer, instead of a CPU pass that could only start after the GPU work and
        // its readback had already finished.
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
            let avg_residual = cpu_kernels::compute_residual(&self.grids[0], &self.solution, &self.rhs);
            println!("Residual sum: {}", avg_residual);
        }
        
    }
}
