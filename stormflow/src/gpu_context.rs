
use stormath::type_aliases::Float;

use wgpu::util::DeviceExt;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue
}

impl GpuContext {
    pub fn new() -> Self {
        pollster::block_on(Self::async_new())
    }

    async fn async_new() -> Self {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        Self{
            device,
            queue
        }
    }

    pub fn byte_length_from_length(length: usize) -> u64 {
        (length * std::mem::size_of::<Float>()) as u64
    }

    pub fn create_shader(&self, shader_src: &str) -> wgpu::ShaderModule {
        self.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(shader_src.into())
            }
        )
    }

    pub fn create_buffer_from_src(&self, content: &[Float]) -> wgpu::Buffer {
        self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(content),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            }
        )
    }

    pub fn create_staging_buffer(&self, length: usize) -> wgpu::Buffer {
        let byte_len = Self::byte_length_from_length(length);

        self.device.create_buffer(
            &wgpu::BufferDescriptor {
                label: None,
                size: byte_len,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        )
    }

    pub fn create_bind_group(
        &self, 
        buffers: &[&wgpu::Buffer], 
        read_only: &[bool]
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
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

        let bind_group_layout = self.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &layout_entries,
            }
        );
        
        let bind_group = self.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &entries,
            }
        );

        (bind_group_layout, bind_group)
    }

    pub fn create_compute_pipeline(
        &self, 
        bind_group_layout: &wgpu::BindGroupLayout, 
        shader: &wgpu::ShaderModule
    ) -> wgpu::ComputePipeline {
        let pipeline_layout = self.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(bind_group_layout)],
                immediate_size: 0,
            }
        );
    
        let pipeline = self.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            }
        );

        pipeline
    }

    pub fn encode_and_submit(
        &self, 
        pipeline: &wgpu::ComputePipeline, 
        bind_group: wgpu::BindGroup,
        storage_buffer: &wgpu::Buffer,
        staging_buffer: &wgpu::Buffer,
        data_length: usize
    ) {
        let byte_len = Self::byte_length_from_length(data_length);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(data_length as u32, 1, 1); // one thread per element
        }
        // Copy GPU result → staging buffer for CPU to read
        encoder.copy_buffer_to_buffer(storage_buffer, 0, staging_buffer, 0, byte_len);
        self.queue.submit([encoder.finish()]);
    }

    pub fn read_from_staging_buffer(&self, staging_buffer: &wgpu::Buffer) -> Vec<Float> {
        let slice = staging_buffer.slice(..);

        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _res = self.device.poll(
            wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(60)),
            }
        );
    
        let result: Vec<f32> = bytemuck::cast_slice(&slice.get_mapped_range()).to_vec();

        result
    }
}