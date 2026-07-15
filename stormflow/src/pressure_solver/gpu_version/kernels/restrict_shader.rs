
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use crate::boundary_conditions::pressure::PressureBoundaryConditions;

use super::bc_consts_wgsl;

const GRID_STRUCT_SRC: &str = include_str!("../../../grid/gpu_version/grid_struct.wgsl");
const RESTRICT_SRC: &str = include_str!("restrict_shader.wgsl");

pub const WORKGROUP_SIZE: u32 = 4;

/// Fuses computing the fine-level residual (rhs - A*x) with restricting it to the coarse level's
/// RHS, mirroring `PressureSolverCPU::compute_residual_and_restrict`.
pub struct RestrictShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl RestrictShader {
    pub fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 5] {
        [
            gpu_utils::uniform_bind_group_layout_entry(0), // grid_fine
            gpu_utils::uniform_bind_group_layout_entry(1), // grid_coarse
            gpu_utils::storage_bind_group_layout_entry(2, true),  // x_fine
            gpu_utils::storage_bind_group_layout_entry(3, true),  // rhs_fine
            gpu_utils::storage_bind_group_layout_entry(4, false), // rhs_coarse
        ]
    }

    /// Boundary conditions are baked into the shader as consts, mirroring `JacobiShader::new`
    /// (see `stencil_terms` in jacobi_shader.wgsl / `residual_at` in restrict_shader.wgsl for why
    /// the fine level's boundary-adjacent cells need the same folded-stencil treatment).
    pub fn new(context: &GpuContext, boundary_conditions: &PressureBoundaryConditions) -> Self {
        let shader_src = format!(
            "{grid_src}\n{bc_consts}{restrict_src}",
            grid_src = GRID_STRUCT_SRC,
            bc_consts = bc_consts_wgsl(boundary_conditions),
            restrict_src = RESTRICT_SRC
        );

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("main", &bind_group_layout, &shader);

        Self {
            pipeline,
            bind_group_layout
        }
    }

    pub fn create_bind_group(
        &self,
        context: &GpuContext,
        grid_fine_buffer: &wgpu::Buffer,
        grid_coarse_buffer: &wgpu::Buffer,
        x_fine_buffer: &wgpu::Buffer,
        rhs_fine_buffer: &wgpu::Buffer,
        rhs_coarse_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        context.create_bind_group(
            &[grid_fine_buffer, grid_coarse_buffer, x_fine_buffer, rhs_fine_buffer, rhs_coarse_buffer],
            &self.bind_group_layout
        )
    }
}
