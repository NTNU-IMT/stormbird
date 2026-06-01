
use crate::gpu_interface::context::GpuContext;

const GRID_SRC: &str = include_str!("../../../grid/gpu_version/grid.wgsl");
const JACOBI_SRC: &str = include_str!("jacobi_shader.wgsl");


pub struct JacobiShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_sol_to_work: wgpu::BindGroup,
    pub bind_group_work_to_sol: wgpu::BindGroup
}

impl JacobiShader {
    pub fn new(
        context: &GpuContext,
        solution_buffer: &wgpu::Buffer,
        work_buffer: &wgpu::Buffer,
        rhs_buffer: &wgpu::Buffer
    ) -> Self {
        let shader_src = format!("{}\n{}", GRID_SRC, JACOBI_SRC);
    
        let shader = context.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(shader_src.into())
            }
        );

        let nr_buffers = 4;
        
        let mut layout_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::with_capacity(nr_buffers);

        let read_only = vec![true, true, false];

        for (index, ro) in read_only.iter().enumerate() {
            layout_entries.push(
                GpuContext::bind_group_layout_entry(index, *ro)
            );
        }

        let bind_group_layout = context.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &layout_entries,
            }
        );

        let pipeline_layout = context.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            }
        );
    
        let pipeline = context.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            }
        );
        
        let mut entries_sol_to_work : Vec<wgpu::BindGroupEntry> = Vec::with_capacity(nr_buffers);

        let buffers_sol_to_work = [
            solution_buffer,
            rhs_buffer,
            work_buffer
        ];

        for (index, buffer) in buffers_sol_to_work.iter().enumerate() {
            entries_sol_to_work.push(
                GpuContext::bind_group_entry(index + 1, buffer)
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
                GpuContext::bind_group_entry(index + 1, buffer)
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