
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use crate::grid::boundary_face::BoundaryFace;

const BOUNDARY_FACE_SRC: &str = include_str!("../../../grid/gpu_version/boundary_face.wgsl");
const GHOST_CELL_SRC: &str = include_str!("set_ghost_cells.wgsl");

pub const WORKGROUP_SIZE: u32 = 64;

pub struct GhostCellShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl GhostCellShader {
    pub fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 3] {
        [
            BoundaryFace::bind_group_layout_entry(0),
            gpu_utils::storage_bind_group_layout_entry(1, false),
            gpu_utils::uniform_bind_group_layout_entry(2),
        ]
    }

    pub fn new(context: &GpuContext) -> Self {
        let shader_src = format!("{}\n{}", BOUNDARY_FACE_SRC, GHOST_CELL_SRC);

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("set_ghost_cells", &bind_group_layout, &shader);

        Self {
            pipeline,
            bind_group_layout
        }
    }

    pub fn create_bind_group(
        &self,
        context: &GpuContext,
        boundary_face_buffer: &wgpu::Buffer,
        p_buffer: &wgpu::Buffer,
        zero_value_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        context.create_bind_group(
            &[boundary_face_buffer, p_buffer, zero_value_buffer],
            &self.bind_group_layout
        )
    }
}
