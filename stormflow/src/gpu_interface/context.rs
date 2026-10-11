
use stormath::type_aliases::Float;

use wgpu::util::DeviceExt;

#[derive(Clone)]
/// Handle to the GPU device and queue. Cloning is cheap (the wgpu handles are reference counted), 
/// and all clones refer to the same device, so buffers can be shared between solvers that are 
/// given clones of the same context.
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
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .unwrap();

        let adapter_info = adapter.get_info();
        println!(
            "[gpu] using adapter '{}' ({:?}, backend={:?})",
            adapter_info.name, adapter_info.device_type, adapter_info.backend
        );

        // Request the adapter's own limits instead of wgpu's conservative
        // (WebGPU-guaranteed-minimum) defaults, so e.g. larger per-level dispatches can cover
        // bigger grids on hardware that supports it.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
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
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            }
        )
    }

    pub fn create_uniform_buffer_u32(&self, value: u32) -> wgpu::Buffer {
        self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&value),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        )
    }

    /// Creates a read-only storage buffer initialized from an arbitrary POD slice (e.g. a packed
    /// array of small descriptor structs), for shaders that need more structured input than a
    /// flat `[Float]` buffer.
    pub fn create_storage_buffer_init<T: bytemuck::Pod>(&self, content: &[T]) -> wgpu::Buffer {
        self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(content),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }
        )
    }

    /// Creates a storage buffer with the given number of `Float` values, initialized to zero, that
    /// can be both read and written by shaders, and copied to and from.
    pub fn create_zeroed_buffer(&self, length: usize) -> wgpu::Buffer {
        self.device.create_buffer(
            &wgpu::BufferDescriptor {
                label: None,
                size: Self::byte_length_from_length(length),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        )
    }

    /// Creates a uniform buffer initialized from a POD value, that can be updated with 
    /// `write_pod_buffer`.
    pub fn create_uniform_buffer_init<T: bytemuck::Pod>(&self, value: &T) -> wgpu::Buffer {
        self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(value),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        )
    }

    pub fn write_pod_buffer<T: bytemuck::Pod>(&self, buffer: &wgpu::Buffer, data: &[T]) {
        self.queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
    }

    /// Copies the first `length` `Float` values of `buffer` to the host. Submits all previously 
    /// recorded work and blocks until the copy is done.
    pub fn read_buffer(&self, buffer: &wgpu::Buffer, length: usize) -> Vec<Float> {
        let staging_buffer = self.create_staging_buffer(length);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(buffer, 0, &staging_buffer, 0, Self::byte_length_from_length(length));
        let submission_index = self.queue.submit([encoder.finish()]);

        self.read_from_staging_buffer(&staging_buffer, submission_index)
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
    
        let result: Vec<f32> = bytemuck::cast_slice(&slice.get_mapped_range().unwrap()).to_vec();

        staging_buffer.unmap();

        result
    }

    pub fn create_bind_group_layout(
        &self, 
        entries: &[wgpu::BindGroupLayoutEntry]
    ) -> wgpu::BindGroupLayout {
        self.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries,
            }
        )
    }

    pub fn create_bind_group(
        &self, 
        buffers: &[&wgpu::Buffer], 
        bind_group_layout: &wgpu::BindGroupLayout
    ) -> wgpu::BindGroup {
        let nr_buffers = buffers.len();

        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::with_capacity(nr_buffers);

        for (index, buffer) in buffers.iter().enumerate() {
            entries.push(
                wgpu::BindGroupEntry {
                    binding: index as u32,
                    resource: buffer.as_entire_binding(),
                }
            );
        }

        self.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: bind_group_layout,
                entries: &entries,
            }
        )
    }

    pub fn create_pipeline(
        &self, 
        entry_point: &str, 
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
    
        self.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            }
        )
    }

    pub fn create_shader_module(&self, shader_src: &str) -> wgpu::ShaderModule {
        self.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(shader_src.into())
            }
        )
    }
}

impl Default for GpuContext {
    fn default() -> Self {
        Self::new()
    }
}
