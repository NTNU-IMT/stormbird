
use stormath::type_aliases::Float;

use super::settings::PressureSolverSettings;

use crate::{
    boundary_conditions::pressure::PressureBoundaryConditions,
    grid::Grid,
    grid::boundary_face::BoundaryFace
};

pub mod kernels;

use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use kernels::jacobi_shader::{JacobiShader, WORKGROUP_SIZE as JACOBI_WORKGROUP_SIZE};
use kernels::ghost_cell_shader::{GhostCellShader, WORKGROUP_SIZE as GHOST_CELL_WORKGROUP_SIZE};
use kernels::restrict_shader::{RestrictShader, WORKGROUP_SIZE as RESTRICT_WORKGROUP_SIZE};
use kernels::prolongate_shader::{ProlongateShader, WORKGROUP_SIZE as PROLONGATE_WORKGROUP_SIZE};

use crate::pressure_solver::cpu_version::kernels as cpu_kernels;

/// All GPU resources belonging to a single multigrid level.
struct GpuLevel {
    grid_buffer: wgpu::Buffer,
    x_buffer: wgpu::Buffer,
    x_work_buffer: wgpu::Buffer,
    rhs_buffer: wgpu::Buffer,
    jacobi_bind_group_sol_to_work: wgpu::BindGroup,
    jacobi_bind_group_work_to_sol: wgpu::BindGroup,
    /// Ghost cell bind groups, one per (axis, face, target buffer) combination. Flattened as
    /// `(axis_index * 2 + face_index) * 2 + buffer_index`, where `buffer_index` is 0 for
    /// `x_buffer` and 1 for `x_work_buffer`.
    ghost_cell_bind_groups: Vec<wgpu::BindGroup>,
    /// Number of cells on each boundary face, indexed by `axis_index * 2 + face_index`.
    ghost_cell_dispatch_totals: [u32; 6],
    /// Workgroup dispatch counts for the Jacobi/interior-sized kernels on this level.
    jacobi_dispatch: [u32; 3],
}

impl GpuLevel {
    #[inline]
    fn ghost_cell_bind_group(&self, axis_index: usize, face_index: usize, buffer_index: usize) -> &wgpu::BindGroup {
        let face_slot = axis_index * 2 + face_index;
        &self.ghost_cell_bind_groups[face_slot * 2 + buffer_index]
    }
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

pub struct PressureSolverGPU {
    pub grids: Vec<Grid>,
    pub boundary_conditions: PressureBoundaryConditions,
    pub solver_settings: PressureSolverSettings,
    pub gpu_context: GpuContext,
    /// Right-hand side for the finest level, on the **interior** grid. Written by the caller
    /// (e.g. `Simulation::pressure_projection_rhs`) and uploaded to the GPU at the start of `solve`.
    pub rhs: Vec<Float>,
    /// Solution for the finest level, on the **extended** grid. Downloaded from the GPU at the
    /// end of `solve`.
    pub solution: Vec<Float>,

    jacobi_shader: JacobiShader,
    ghost_cell_shader: GhostCellShader,
    restrict_shader: RestrictShader,
    prolongate_shader: ProlongateShader,

    levels: Vec<GpuLevel>,
    restrict_levels: Vec<RestrictLevel>,
    prolongate_levels: Vec<ProlongateLevel>,

    /// Uniform buffers holding the GPU ghost-cell flag (0 = ZeroGradient, 1 = ZeroValue) for
    /// each (axis, face). Shared across all levels since the boundary condition config doesn't
    /// depend on grid resolution.
    #[allow(dead_code)]
    zero_value_buffers: [[wgpu::Buffer; 2]; 3],

    solution_staging_buffer: wgpu::Buffer,
}

impl PressureSolverGPU {
    pub fn new(
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
        solver_settings: PressureSolverSettings
    ) -> Self {
        let grids = grid.multigrid_hierarchy();
        let nr_levels = grids.len();

        let gpu_context = GpuContext::new();

        let jacobi_shader = JacobiShader::new(&gpu_context);
        let ghost_cell_shader = GhostCellShader::new(&gpu_context);
        let restrict_shader = RestrictShader::new(&gpu_context);
        let prolongate_shader = ProlongateShader::new(&gpu_context);

        // Zero-value uniform buffers only depend on the boundary condition configuration, not
        // on grid resolution, so they're shared across all levels.
        let zero_value_buffers: [[wgpu::Buffer; 2]; 3] = std::array::from_fn(|axis_index| {
            std::array::from_fn(|face_index| {
                let flag = boundary_conditions.condition(axis_index, face_index).as_gpu_flag();
                gpu_context.create_uniform_buffer_u32(flag)
            })
        });

        let mut levels: Vec<GpuLevel> = Vec::with_capacity(nr_levels);

        for level_grid in &grids {
            let grid_buffer = level_grid.as_gpu_version().as_buffer(&gpu_context);

            let x_host = vec![0.0 as Float; level_grid.nr_extended_cells()];
            let rhs_host = vec![0.0 as Float; level_grid.nr_interior_cells()];

            let x_buffer = gpu_context.create_buffer_from_src(&x_host);
            let x_work_buffer = gpu_context.create_buffer_from_src(&x_host);
            let rhs_buffer = gpu_context.create_buffer_from_src(&rhs_host);

            let (jacobi_bind_group_sol_to_work, jacobi_bind_group_work_to_sol) = jacobi_shader.create_bind_groups(
                &gpu_context,
                &grid_buffer,
                &x_buffer,
                &rhs_buffer,
                &x_work_buffer
            );

            let mut ghost_cell_bind_groups: Vec<wgpu::BindGroup> = Vec::with_capacity(12);
            let mut ghost_cell_dispatch_totals = [0u32; 6];

            for axis_index in 0..3 {
                for face_index in 0..2 {
                    let boundary_face = BoundaryFace::new(
                        level_grid.extended_shape,
                        level_grid.extended_stride,
                        axis_index,
                        face_index
                    );

                    let boundary_face_buffer = boundary_face.as_buffer(&gpu_context);
                    ghost_cell_dispatch_totals[axis_index * 2 + face_index] =
                        boundary_face.shape[0] * boundary_face.shape[1];

                    let zero_value_buffer = &zero_value_buffers[axis_index][face_index];

                    ghost_cell_bind_groups.push(
                        ghost_cell_shader.create_bind_group(&gpu_context, &boundary_face_buffer, &x_buffer, zero_value_buffer)
                    );
                    ghost_cell_bind_groups.push(
                        ghost_cell_shader.create_bind_group(&gpu_context, &boundary_face_buffer, &x_work_buffer, zero_value_buffer)
                    );
                }
            }

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
                ghost_cell_bind_groups,
                ghost_cell_dispatch_totals,
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

        let solution = vec![0.0 as Float; grids[0].nr_extended_cells()];
        let rhs = vec![0.0 as Float; grids[0].nr_interior_cells()];
        let solution_staging_buffer = gpu_context.create_staging_buffer(solution.len());

        Self {
            grids,
            boundary_conditions: boundary_conditions.clone(),
            solver_settings,
            gpu_context,
            rhs,
            solution,
            jacobi_shader,
            ghost_cell_shader,
            restrict_shader,
            prolongate_shader,
            levels,
            restrict_levels,
            prolongate_levels,
            zero_value_buffers,
            solution_staging_buffer,
        }
    }

    /// Sets the ghost cells of the given level's buffer (0 = solution, 1 = work) for all six
    /// boundary faces, in the same axis/face order used by `PressureBoundaryConditions::set_ghost_cells`
    /// on the CPU (so corner/edge cells that depend on a previously-updated face are consistent).
    fn set_ghost_cells_gpu(&self, encoder: &mut wgpu::CommandEncoder, level: usize, buffer_index: usize) {
        let level_data = &self.levels[level];

        for axis_index in 0..3 {
            for face_index in 0..2 {
                let bind_group = level_data.ghost_cell_bind_group(axis_index, face_index, buffer_index);
                let total = level_data.ghost_cell_dispatch_totals[axis_index * 2 + face_index];

                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&self.ghost_cell_shader.pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch_workgroups(
                    gpu_utils::workgroup_count(total as usize, GHOST_CELL_WORKGROUP_SIZE as usize),
                    1,
                    1
                );
            }
        }
    }

    fn poisson_jacobi_smoother_gpu(
        &self, 
        encoder: &mut wgpu::CommandEncoder, 
        level: usize, 
        nr_iterations: usize
    ) {
        let level_data = &self.levels[level];

        // Mirrors PressureSolverCPU::poisson_jacobi_smoother: seed the work buffer from the
        // solution buffer, then set ghost cells on both, since the first iteration reads from
        // the solution buffer and later odd/even iterations alternate.
        let byte_len = GpuContext::byte_length_from_length(self.grids[level].nr_extended_cells());
        encoder.copy_buffer_to_buffer(&level_data.x_buffer, 0, &level_data.x_work_buffer, 0, byte_len);

        self.set_ghost_cells_gpu(encoder, level, 0);
        self.set_ghost_cells_gpu(encoder, level, 1);

        for iteration in 0..nr_iterations {
            let (bind_group, written_buffer_index) = if iteration % 2 == 0 {
                (&level_data.jacobi_bind_group_sol_to_work, 1usize)
            } else {
                (&level_data.jacobi_bind_group_work_to_sol, 0usize)
            };

            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&self.jacobi_shader.pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.dispatch_workgroups(
                    level_data.jacobi_dispatch[0],
                    level_data.jacobi_dispatch[1],
                    level_data.jacobi_dispatch[2]
                );
            }

            self.set_ghost_cells_gpu(encoder, level, written_buffer_index);
        }

        // If odd number of iterations, the result is in the work buffer; copy back to solution.
        if nr_iterations % 2 == 1 {
            let byte_len = GpuContext::byte_length_from_length(self.grids[level].nr_extended_cells());
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
        self.poisson_jacobi_smoother_gpu(encoder, nr_levels - 1, nr_iterations * 4);

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
    /// grid) before calling this. On return, `self.solution` holds the finest-level solution
    /// on the extended grid.
    pub fn solve(&mut self) {
        self.gpu_context.write_buffer(&self.levels[0].rhs_buffer, &self.rhs);

        let mut encoder = self.gpu_context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default()
        );

        for _ in 0..self.solver_settings.nr_v_cycles {
            self.perform_v_cycle(&mut encoder);
        }

        let byte_len = GpuContext::byte_length_from_length(self.solution.len());
        encoder.copy_buffer_to_buffer(&self.levels[0].x_buffer, 0, &self.solution_staging_buffer, 0, byte_len);

        let submission_index = self.gpu_context.queue.submit([encoder.finish()]);

        self.solution = self.gpu_context.read_from_staging_buffer(&self.solution_staging_buffer, submission_index);

        let avg_residual = cpu_kernels::compute_residual(&self.grids[0], &self.solution, &self.rhs);
        println!("Residual sum: {}", avg_residual);
    }
}
