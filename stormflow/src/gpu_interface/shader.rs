
use super::context::GpuContext;

pub struct ComputeShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
}

impl ComputeShader {
    pub fn new(
        context: &GpuContext,
        buffers: &[&wgpu::Buffer], 
        read_only: &[bool],
        shader_src: &str
    ) -> Self {
        let shader = context.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(shader_src.into())
            }
        );

        let nr_buffers = buffers.len();

        let mut layout_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::with_capacity(nr_buffers);
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::with_capacity(nr_buffers);

        for i in 0..nr_buffers {
            layout_entries.push(
                wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: read_only[i] },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            );

            entries.push(
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers[i].as_entire_binding(),
                }
            );
        }

        let bind_group_layout = context.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &layout_entries,
            }
        );
        
        let bind_group = context.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &entries,
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

        Self {
            pipeline,
            bind_group
        }
    }
}