
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

const GPU_GRID_SRC: &str = include_str!("../../../grid/gpu_version/gpu_grid.wgsl");
const PROLONGATE_SRC: &str = include_str!("prolongate_shader.wgsl");

pub const WORKGROUP_SIZE: u32 = 4;

/// Trilinearly interpolates the coarse-level solution and adds it directly to the fine-level
/// solution, mirroring `PressureSolverCPU::prolongate_and_correct`.
pub struct ProlongateShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl ProlongateShader {
    pub fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 4] {
        [
            gpu_utils::uniform_bind_group_layout_entry(0), // grid_fine
            gpu_utils::uniform_bind_group_layout_entry(1), // grid_coarse
            gpu_utils::storage_bind_group_layout_entry(2, true),  // x_coarse
            gpu_utils::storage_bind_group_layout_entry(3, false), // x_fine
        ]
    }

    pub fn new(context: &GpuContext) -> Self {
        let shader_src = format!("{}\n{}", GPU_GRID_SRC, PROLONGATE_SRC);

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
        x_coarse_buffer: &wgpu::Buffer,
        x_fine_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        context.create_bind_group(
            &[grid_fine_buffer, grid_coarse_buffer, x_coarse_buffer, x_fine_buffer],
            &self.bind_group_layout
        )
    }
}
