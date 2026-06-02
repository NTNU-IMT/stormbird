
use crate::gpu_interface::context::GpuContext;

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuGrid {
    pub start_point: [f32; 4],
    pub cell_length: [f32; 4],
    pub inv_cell_length: [f32; 4],
    pub inv_cell_length_squared: [f32; 4],
    pub poisson_diagonal: f32,
    pub poisson_inv_diagonal: f32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub extended_shape: [u32; 4],
    pub extended_stride: [u32; 4],
    pub interior_shape: [u32; 4],
    pub interior_stride: [u32; 4],
    
}

impl GpuGrid {
    pub fn as_buffer(&self, context: &GpuContext) -> wgpu::Buffer {
        context.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Grid Uniform Buffer"),
                contents: bytemuck::cast_slice(&[*self]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        )
    }

    pub fn bind_group_layout_entry(binding: usize) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: binding as u32,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }
}
