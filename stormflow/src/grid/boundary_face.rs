
use crate::gpu_interface::context::GpuContext;

use wgpu::util::DeviceExt;

/// The remaining axes in a plane, for each axis index
/// 
/// - x-boundaries has the axes y and z (1, 2)
/// - y-boundaries has the axes x and z (0, 2)
/// - z-boundaries has the axes x and y (0, 1)
const PLANE_AXIS: [(usize, usize); 3] = [
    (1, 2),
    (0, 2),
    (0, 1)
];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
/// Structure with the necessary information to be able to iterate over a boundary face. The primary
/// purpose is to be able to apply boundary conditions effectively
pub struct BoundaryFace {
    /// Generally either 0 or the length of axis - 1
    pub axis_offset: u32,
    /// Delta value to be able to access the neighbor cell
    pub neighbor_delta: i32,
    /// Shape of the face
    pub shape: [u32; 2],
    /// Stride in the local axis, to be able to iterate over the face
    pub stride: [u32; 2],
}

impl BoundaryFace {
    pub fn new(
        grid_shape: [usize; 3],
        grid_stride: [usize; 3],
        axis_index: usize,
        face_index: usize,
    ) -> Self {
        let axis_length = grid_shape[axis_index] as u32;
        let axis_stride = grid_stride[axis_index] as u32;

        let (plane_axes_0, plane_axes_1) = PLANE_AXIS[axis_index];
        let (inner_axis, outer_axis) = if grid_stride[plane_axes_0] <= grid_stride[plane_axes_1] {
            (plane_axes_0, plane_axes_1)
        } else {
            (plane_axes_1, plane_axes_0)
        };

        let shape = [grid_shape[outer_axis] as u32, grid_shape[inner_axis] as u32];
        let stride = [grid_stride[outer_axis] as u32, grid_stride[inner_axis] as u32];

        let (axis_offset, neighbor_delta) = if face_index == 0 {
            // current layer 0, neighbor layer 1 -> neighbor = current + axis_stride
            (0, axis_stride as i32)
        } else {
            // current layer L-1, neighbor layer L-2 -> neighbor = current - axis_stride
            ((axis_length - 1) * axis_stride, -(axis_stride as i32))
        };

        Self {
            axis_offset,
            neighbor_delta,
            shape,
            stride
        }
    }

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
