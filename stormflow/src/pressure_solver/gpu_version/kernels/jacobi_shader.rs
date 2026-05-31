

const SHADER_SRC: &str = include_str!("jacobi_shader.wgsl");

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
        let shader = context.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into())
            }
        );

        let nr_buffers = 3;
        
        let mut layout_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::with_capacity(nr_buffers);
        let read_only = vec![true, true, false]

        for ro in read_only.iter() {
            layout_entries.push(
                wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: ro },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
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
        
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::with_capacity(nr_buffers);
    }
}