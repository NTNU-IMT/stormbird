
use stormath::type_aliases::Float;

use super::settings::PressureSolverSettings;

use crate::{
    boundary_conditions::pressure::PressureBoundaryConditions, 
    grid::Grid
};

pub mod kernels;

use crate::gpu_interface::context::GpuContext;

use kernels::jacobi_shader::JacobiShader;

pub struct PressureSolverGPU {
    pub grid: Grid,
    pub solution: Vec<Float>,
    pub rhs: Vec<Float>,
    pub boundary_conditions: PressureBoundaryConditions,
    pub solver_settings: PressureSolverSettings,
    pub gpu_context: GpuContext,
    pub shader: JacobiShader,
    pub solution_buffer: wgpu::Buffer,
    pub solution_work_buffer: wgpu::Buffer,
    pub solution_staging_buffer: wgpu::Buffer,
    pub rhs_buffer: wgpu::Buffer
}

impl PressureSolverGPU {
    pub fn new(
        grid: &Grid, 
        boundary_conditions: &PressureBoundaryConditions, 
        solver_settings: PressureSolverSettings
    ) -> Self {
        let solution = vec![0.0; grid.nr_extended_cells()];
        let rhs = vec![0.0; grid.nr_interior_cells()];

        let gpu_context = GpuContext::new();

        let solution_buffer = gpu_context.create_buffer_from_src(&solution);
        let solution_work_buffer = gpu_context.create_buffer_from_src(&solution);

        let solution_staging_buffer = gpu_context.create_staging_buffer(solution.len());

        let rhs_buffer = gpu_context.create_buffer_from_src(&rhs);

        let shader = JacobiShader::new(
            &gpu_context,
            &solution_buffer,
            &solution_work_buffer,
            &rhs_buffer
        );
        
        Self {
            grid: grid.clone(),
            solution,
            rhs,
            boundary_conditions: boundary_conditions.clone(),
            solver_settings: solver_settings.clone(),
            gpu_context,
            shader,
            solution_buffer,
            solution_work_buffer,
            solution_staging_buffer,
            rhs_buffer
        }
    }

    pub fn solve(&mut self) {
        self.gpu_context.write_buffer(&self.rhs_buffer, &self.rhs);

        let mut encoder = self.gpu_context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default()
        );

        let nr_iterations = 10;
        
        for i in 0..nr_iterations {
            let bind_group = if i % 2 == 0 {
                &self.shader.bind_group_sol_to_work
            } else {
                &self.shader.bind_group_work_to_sol
            };
            
            let mut pass = encoder.begin_compute_pass(
                &wgpu::ComputePassDescriptor::default()
            );
            
            pass.set_pipeline(&self.shader.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(4, 1, 1);
        }
        
        let byte_len = GpuContext::byte_length_from_length(self.solution.len());

        encoder.copy_buffer_to_buffer(
            &self.solution_buffer, 
            0, 
            &self.solution_staging_buffer, 
            0, 
            byte_len
        );

        let submission_index = self.gpu_context.queue.submit([encoder.finish()]);

        let slice = self.solution_staging_buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _res = self.gpu_context.device.poll(
            wgpu::PollType::Wait {
                submission_index: Some(submission_index),
                timeout: Some(std::time::Duration::from_secs(60)),
            }
        );
    
        self.solution = bytemuck::cast_slice(&slice.get_mapped_range()).to_vec();
    }
}