
use stormath::type_aliases::Float;

use super::settings::PressureSolverSettings;

use crate::{
    boundary_conditions::pressure::PressureBoundaryConditions, 
    grid::Grid
};

use crate::gpu_context::GpuContext;

const JACOBI_SHADER: &str = include_str!("kernels/jacobi_smoother.wgsl");

pub struct PressureSolverGPU {
    pub grid: Grid,
    pub x: Vec<Float>,
    pub rhs: Vec<Float>,
    pub boundary_conditions: PressureBoundaryConditions,
    pub solver_settings: PressureSolverSettings,
    pub gpu_context: GpuContext,
    pub shader: wgpu::ShaderModule
}

impl PressureSolverGPU {
    pub fn new(
        grid: &Grid, 
        boundary_conditions: &PressureBoundaryConditions, 
        solver_settings: PressureSolverSettings
    ) -> Self {
        let x = vec![0.0; grid.nr_extended_cells()];
        let rhs = vec![0.0; grid.nr_interior_cells()];

        let gpu_context = GpuContext::new();

        let shader = gpu_context.create_shader(JACOBI_SHADER);

        Self {
            grid: grid.clone(),
            x,
            rhs,
            boundary_conditions: boundary_conditions.clone(),
            solver_settings: solver_settings.clone(),
            gpu_context,
            shader
        }
    }
}