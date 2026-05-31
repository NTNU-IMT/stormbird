
use stormath::type_aliases::Float;

use wgpu::util::DeviceExt;

use super::shader::ComputeShader;

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

    pub fn create_buffer_from_src(&self, content: &[Float]) -> wgpu::Buffer {
        self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(content),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            }
        )
    }

    pub fn write_buffer(&self, buffer: &wgpu::Buffer, data: &[Float]) {
        let offset = 0;
        self.queue.write_buffer(
            buffer, 
            offset, 
            bytemuck::cast_slice(data)
        );
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

    pub fn run_compute_shader(
        &self, 
        shader: &ComputeShader,
        storage_buffer: &wgpu::Buffer,
        staging_buffer: &wgpu::Buffer,
        data_length: usize
    ) -> wgpu::SubmissionIndex {
        let byte_len = Self::byte_length_from_length(data_length);

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default()
        );
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&shader.pipeline);
            pass.set_bind_group(0, &shader.bind_group, &[]);
            pass.dispatch_workgroups(data_length as u32, 1, 1); // one thread per element
        }
        // Copy GPU result → staging buffer for CPU to read
        encoder.copy_buffer_to_buffer(storage_buffer, 0, staging_buffer, 0, byte_len);
        self.queue.submit([encoder.finish()])
    }

    pub fn read_from_staging_buffer(
        &self, 
        staging_buffer: &wgpu::Buffer, 
        index: wgpu::SubmissionIndex
    ) -> Vec<Float> {
        let slice = staging_buffer.slice(..);

        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _res = self.device.poll(
            wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: Some(std::time::Duration::from_secs(60)),
            }
        );
    
        let result: Vec<f32> = bytemuck::cast_slice(&slice.get_mapped_range()).to_vec();

        result
    }
}