
use stormath::type_aliases::Float;

use super::settings::PressureSolverSettings;

use crate::{
    boundary_conditions::pressure::PressureBoundaryConditions, 
    grid::Grid
};

use crate::gpu_interface::context::GpuContext;
use crate::gpu_interface::shader::ComputeShader;

const JACOBI_SHADER: &str = include_str!("kernels/jacobi_smoother.wgsl");

pub struct PressureSolverGPU {
    pub grid: Grid,
    pub solution: Vec<Float>,
    pub rhs: Vec<Float>,
    pub boundary_conditions: PressureBoundaryConditions,
    pub solver_settings: PressureSolverSettings,
    pub gpu_context: GpuContext,
    pub shader: ComputeShader,
    pub solution_buffer: wgpu::Buffer,
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

        let solution_staging_buffer = gpu_context.create_staging_buffer(solution.len());

        let rhs_buffer = gpu_context.create_buffer_from_src(&rhs);

        let buffers = [
            &rhs_buffer, 
            &solution_buffer,
        ];

        let read_only = [true, false];

        let shader = ComputeShader::new(
            &gpu_context,
            &buffers,
            &read_only,
            JACOBI_SHADER
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
            solution_staging_buffer,
            rhs_buffer
        }
    }

    pub fn solve(&mut self) {
        self.gpu_context.write_buffer(&self.rhs_buffer, &self.rhs);

        let index = self.gpu_context.run_compute_shader(
            &self.shader, 
            &self.solution_buffer, 
            &self.solution_staging_buffer, 
            self.solution.len()
        );

        let solution = self.gpu_context.read_from_staging_buffer(
            &self.solution_staging_buffer,
            index
        );

        self.solution.copy_from_slice(&solution);
    }
}