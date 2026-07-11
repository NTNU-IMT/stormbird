
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use crate::grid::Grid;
use crate::boundary_conditions::pressure::PressureBoundaryConditions;

const GHOST_CELL_SRC: &str = include_str!("set_ghost_cells.wgsl");

pub const WORKGROUP_SIZE: u32 = 64;

/// The remaining axes in a plane, for each axis index (mirrors `grid::boundary_face::PLANE_AXIS`).
const PLANE_AXIS: [(usize, usize); 3] = [
    (1, 2),
    (0, 2),
    (0, 1)
];

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FaceDescriptorGpu {
    axis_offset: u32,
    neighbor_delta: i32,
    shape: [u32; 2],
    stride: [u32; 2],
    zero_value: u32,
    start_offset: u32,
}

pub struct GhostCellShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl GhostCellShader {
    pub fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
        [
            gpu_utils::storage_bind_group_layout_entry(0, true),  // faces
            gpu_utils::storage_bind_group_layout_entry(1, false), // p
        ]
    }

    pub fn new(context: &GpuContext) -> Self {
        let shader = context.create_shader_module(GHOST_CELL_SRC);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("set_ghost_cells", &bind_group_layout, &shader);

        Self {
            pipeline,
            bind_group_layout
        }
    }

    pub fn create_bind_group(
        &self,
        context: &GpuContext,
        faces_buffer: &wgpu::Buffer,
        p_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        context.create_bind_group(
            &[faces_buffer, p_buffer],
            &self.bind_group_layout
        )
    }

    /// Builds the packed 6-face descriptor buffer for a single grid level, restricted to the
    /// "core" of each face (the part where the two transverse indices are within the interior
    /// range). This is the only part any of the pressure solver's stencils ever read — the
    /// Laplacian/Jacobi/restrict/prolongate kernels always evaluate a neighbor exactly one cell
    /// off a genuine interior cell along a single axis, so the transverse indices are always in
    /// range. Corner/edge ghost cells are therefore never consumed and are intentionally skipped,
    /// which also makes all 6 faces independent of each other (no face's core ever reads another
    /// face's ghost values), so they can be resolved by a single flat dispatch.
    ///
    /// Returns the uploaded storage buffer and the total number of core cells across all 6 faces
    /// (i.e. the dispatch size).
    pub fn build_faces_buffer(
        context: &GpuContext,
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
    ) -> (wgpu::Buffer, u32) {
        let mut descriptors: Vec<FaceDescriptorGpu> = Vec::with_capacity(6);
        let mut start_offset: u32 = 0;

        for axis_index in 0..3 {
            for face_index in 0..2 {
                let axis_length = grid.extended_shape[axis_index] as u32;
                let axis_stride = grid.extended_stride[axis_index] as u32;

                let (plane_axis_0, plane_axis_1) = PLANE_AXIS[axis_index];
                let (inner_axis, outer_axis) = if grid.extended_stride[plane_axis_0] <= grid.extended_stride[plane_axis_1] {
                    (plane_axis_0, plane_axis_1)
                } else {
                    (plane_axis_1, plane_axis_0)
                };

                // Core-only shape/stride: the transverse axes are restricted to their interior
                // range instead of the full extended range.
                let shape = [grid.interior_shape[outer_axis] as u32, grid.interior_shape[inner_axis] as u32];
                let stride = [grid.extended_stride[outer_axis] as u32, grid.extended_stride[inner_axis] as u32];

                let (face_axis_offset, neighbor_delta) = if face_index == 0 {
                    (0u32, axis_stride as i32)
                } else {
                    ((axis_length - 1) * axis_stride, -(axis_stride as i32))
                };

                // Shift past the transverse ghost layer (index 0) to the first core cell (index 1).
                let axis_offset = face_axis_offset + stride[0] + stride[1];

                let zero_value = boundary_conditions.condition(axis_index, face_index).as_gpu_flag();
                let count = shape[0] * shape[1];

                descriptors.push(FaceDescriptorGpu {
                    axis_offset,
                    neighbor_delta,
                    shape,
                    stride,
                    zero_value,
                    start_offset,
                });

                start_offset += count;
            }
        }

        let faces_buffer = context.create_storage_buffer_init(&descriptors);

        (faces_buffer, start_offset)
    }
}
