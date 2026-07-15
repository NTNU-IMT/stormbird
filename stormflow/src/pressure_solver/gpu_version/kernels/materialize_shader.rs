
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use crate::grid::gpu_version::GpuGrid;
use crate::boundary_conditions::pressure::PressureBoundaryConditions;

use super::bc_consts_wgsl;

const GRID_SRC: &str = include_str!("../../../grid/gpu_version/grid.wgsl");
const MATERIALIZE_SRC: &str = include_str!("materialize_shader.wgsl");

pub const WORKGROUP_SIZE: u32 = 4;

/// Materializes the interior-only solve result into the extended layout `update_velocity`/
/// `export_fields_as_vtk` expect (interior values at their offset position, plus each
/// boundary-adjacent cell's extrapolated ghost value), as a single GPU dispatch appended to the
/// same command buffer as the solve — replacing what would otherwise be a CPU pass run only after
/// the GPU readback completes.
pub struct MaterializeShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl MaterializeShader {
    pub fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 3] {
        [
            GpuGrid::bind_group_layout_entry(0),
            gpu_utils::storage_bind_group_layout_entry(1, true),  // x (interior)
            gpu_utils::storage_bind_group_layout_entry(2, false), // solution (extended)
        ]
    }

    pub fn new(context: &GpuContext, boundary_conditions: &PressureBoundaryConditions) -> Self {
        let shader_src = format!(
            "{grid_src}\n{bc_consts}{materialize_src}",
            grid_src = GRID_SRC,
            bc_consts = bc_consts_wgsl(boundary_conditions),
            materialize_src = MATERIALIZE_SRC
        );

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("main", &bind_group_layout, &shader);

        Self {
            pipeline,
            bind_group_layout
        }
    }

    pub fn create_bind_group(
        &self,
        context: &GpuContext,
        grid_buffer: &wgpu::Buffer,
        x_buffer: &wgpu::Buffer,
        solution_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        context.create_bind_group(
            &[grid_buffer, x_buffer, solution_buffer],
            &self.bind_group_layout
        )
    }
}
