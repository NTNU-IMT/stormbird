
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
            GpuGrid::bind_group_entry(0),
            gpu_utils::storage_bind_group_layout_entry(1, true),
            gpu_utils::storage_bind_group_layout_entry(2, true)
            gpu_utils::storage_bind_group_layout_entry(3, false)
        ]
    }
    
    pub fn new(
        context: &GpuContext,
        solution_buffer: &wgpu::Buffer,
        work_buffer: &wgpu::Buffer,
        rhs_buffer: &wgpu::Buffer
    ) -> Self {
        let shader_src = format!("{}\n{}", GRID_SRC, JACOBI_SRC);
    
        let shader = context.create_shader_module(&shader_src);
        let layout_entries = Self::bind_group_layout_entries();
        let bind_group_layout = context.create_bind_group_layout(&layout_entries);
        let pipeline = context.create_pipeline("main", &bind_group_layout);
        
        
        let mut entries_sol_to_work : Vec<wgpu::BindGroupEntry> = Vec::with_capacity(nr_buffers);

        let buffers_sol_to_work = [
            solution_buffer,
            rhs_buffer,
            work_buffer
        ];

        for (index, buffer) in buffers_sol_to_work.iter().enumerate() {
            entries_sol_to_work.push(
                gpu_utils::bind_group_entry(index + 1, buffer)
            );
        }

        let mut entries_work_to_sol : Vec<wgpu::BindGroupEntry> = Vec::with_capacity(nr_buffers);

        let buffers_work_to_sol = [
            work_buffer,
            rhs_buffer,
            solution_buffer
        ];

        for (index, buffer) in buffers_work_to_sol.iter().enumerate() {
            entries_work_to_sol.push(
                gpu_utils::bind_group_entry(index + 1, buffer)
            );
        }
        
        let bind_group_sol_to_work = context.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &entries_sol_to_work,
            }
        );

        let bind_group_work_to_sol = context.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &entries_work_to_sol,
            }
        );

        Self {
            pipeline,
            bind_group_sol_to_work,
            bind_group_work_to_sol
        }
    }
}
