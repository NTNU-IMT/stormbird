
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

const GRID_SRC: &str = include_str!("../../../grid/gpu_version/grid.wgsl");
const JACOBI_SRC: &str = include_str!("jacobi_shader.wgsl");

use crate::grid::gpu_version::GpuGrid;

pub struct JacobiShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_sol_to_work: wgpu::BindGroup,
    pub bind_group_work_to_sol: wgpu::BindGroup
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
    
    pub fn new(
        context: &GpuContext,
        grid_buffer: &wgpu::Buffer,
        solution_buffer: &wgpu::Buffer,
        rhs_buffer: &wgpu::Buffer,
        work_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader_src = format!("{}\n{}", GRID_SRC, JACOBI_SRC);
    
        let shader = context.create_shader_module(&shader_src);
        let layout_entries = Self::bind_group_layout_entries();
        let bind_group_layout = context.create_bind_group_layout(&layout_entries);
        let pipeline = context.create_pipeline("main", &bind_group_layout, &shader);

        let buffers_sol_to_work = [
            grid_buffer,
            solution_buffer,
            rhs_buffer,
            work_buffer
        ];

        let buffers_work_to_sol = [
            grid_buffer,
            work_buffer,
            rhs_buffer,
            solution_buffer
        ];
        
        let bind_group_sol_to_work = context.create_bind_group(
            &buffers_sol_to_work, 
            &bind_group_layout
        );

        let bind_group_work_to_sol = context.create_bind_group(
            &buffers_work_to_sol,
            &bind_group_layout
        );

        Self {
            pipeline,
            bind_group_sol_to_work,
            bind_group_work_to_sol
        }
    }
}
