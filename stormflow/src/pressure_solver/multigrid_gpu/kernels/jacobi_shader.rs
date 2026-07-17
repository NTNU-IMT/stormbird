
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

const GRID_SRC: &str = include_str!("../../../grid/gpu_version/grid.wgsl");
const JACOBI_SRC: &str = include_str!("jacobi_shader.wgsl");

use crate::grid::gpu_version::GpuGrid;
use crate::pressure_solver::boundary_conditions::PressureBoundaryConditions;

use super::bc_consts_wgsl;

pub const WORKGROUP_SIZE: u32 = 4;

pub struct JacobiShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl JacobiShader {
    pub fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 4] {
        [
            GpuGrid::bind_group_layout_entry(0),
            gpu_utils::storage_bind_group_layout_entry(1, true),
            gpu_utils::storage_bind_group_layout_entry(2, true),
            gpu_utils::storage_bind_group_layout_entry(3, false)
        ]
    }

    /// Boundary conditions are baked into the shader as consts (rather than passed via a buffer)
    /// since they're fixed for the solver's lifetime and don't depend on grid resolution, so this
    /// single pipeline is shared across every multigrid level's bind groups.
    pub fn new(context: &GpuContext, boundary_conditions: &PressureBoundaryConditions) -> Self {
        let shader_src = format!(
            "{grid_src}\n{bc_consts}{jacobi_src}",
            grid_src = GRID_SRC,
            bc_consts = bc_consts_wgsl(boundary_conditions),
            jacobi_src = JACOBI_SRC
        );

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("main", &bind_group_layout, &shader);

        Self {
            pipeline,
            bind_group_layout
        }
    }

    /// Creates the pair of bind groups needed to alternate the Jacobi smoother between the
    /// solution and work buffers of a single grid level.
    pub fn create_bind_groups(
        &self,
        context: &GpuContext,
        grid_buffer: &wgpu::Buffer,
        solution_buffer: &wgpu::Buffer,
        rhs_buffer: &wgpu::Buffer,
        work_buffer: &wgpu::Buffer,
    ) -> (wgpu::BindGroup, wgpu::BindGroup) {
        let bind_group_sol_to_work = context.create_bind_group(
            &[grid_buffer, solution_buffer, rhs_buffer, work_buffer],
            &self.bind_group_layout
        );

        let bind_group_work_to_sol = context.create_bind_group(
            &[grid_buffer, work_buffer, rhs_buffer, solution_buffer],
            &self.bind_group_layout
        );

        (bind_group_sol_to_work, bind_group_work_to_sol)
    }
}
