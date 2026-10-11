
use crate::gpu_interface::context::GpuContext;
use crate::grid::INTERIOR_OFFSET;

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
    /// `ghost_layer` selects which of the `INTERIOR_OFFSET` ghost layers on this face to
    /// address: `0` is the layer immediately adjacent to the interior, `INTERIOR_OFFSET - 1` is
    /// the outermost layer. Each layer is paired with the interior cell that mirrors it across
    /// the domain boundary face (layer `0` with the nearest interior cell, layer `1` with the
    /// next one in, and so on), so `neighbor_delta` always points at an interior cell, never at
    /// another ghost layer — callers can therefore fill all `INTERIOR_OFFSET` layers in any
    /// order.
    pub fn new(
        grid_shape: [usize; 3],
        grid_stride: [usize; 3],
        axis_index: usize,
        face_index: usize,
        ghost_layer: usize,
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

        // Cell distance between this ghost layer and its mirrored interior neighbor: layer 0
        // (nearest the interior) pairs with the nearest interior cell (distance 1), layer 1
        // with the next interior cell in (distance 3), and so on.
        let neighbor_distance = (2 * ghost_layer + 1) as u32;

        let (axis_offset, neighbor_delta) = if face_index == 0 {
            let offset = (INTERIOR_OFFSET - 1 - ghost_layer) as u32;

            (offset * axis_stride, (neighbor_distance * axis_stride) as i32)
        } else {
            let offset = axis_length - INTERIOR_OFFSET as u32 + ghost_layer as u32;

            (offset * axis_stride, -((neighbor_distance * axis_stride) as i32))
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
