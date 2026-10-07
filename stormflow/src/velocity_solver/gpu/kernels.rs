use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};
use crate::grid::INTERIOR_OFFSET;

const GPU_GRID_SRC: &str = include_str!("../../grid/gpu_version/gpu_grid.wgsl");
const COMMON_SRC: &str = include_str!("shaders/common.wgsl");

pub const CONVECT_AND_DIFFUSE_SRC: &str = include_str!("shaders/convect_and_diffuse.wgsl");
pub const PRESSURE_RHS_SRC: &str = include_str!("shaders/pressure_rhs.wgsl");
pub const ADD_PRESSURE_GRADIENT_SRC: &str = include_str!("shaders/add_pressure_gradient.wgsl");
pub const NO_SLIP_CORRECTION_SRC: &str = include_str!("shaders/no_slip_correction.wgsl");
pub const SLIP_CORRECTION_SRC: &str = include_str!("shaders/slip_correction.wgsl");
pub const GHOST_CELLS_SRC: &str = include_str!("shaders/ghost_cells.wgsl");
pub const MAX_VELOCITY_SRC: &str = include_str!("shaders/max_velocity.wgsl");
pub const CELL_SAMPLING_SRC: &str = include_str!("shaders/cell_sampling.wgsl");

/// Workgroup size for kernels over the interior cells. The first dimension runs along the
/// contiguous z-axis of the grid, to get coalesced memory access.
pub const WG_3D: [u32; 3] = [16, 4, 4];
/// Workgroup size for kernels over 1D lists
pub const WG_1D: u32 = 64;
/// Workgroup size, in each dimension, for kernels over boundary faces
pub const WG_PLANE: u32 = 16;
/// Workgroup size for the reduction kernel
pub const WG_REDUCE: u32 = 256;
/// Max number of workgroups for the reduction kernel. Each invocation loops over the cells it
/// needs to cover beyond this.
pub const MAX_REDUCE_WORKGROUPS: u32 = 4096;

/// Max number of workgroups in one dispatch dimension, guaranteed by the WebGPU spec.
const MAX_WORKGROUPS_PER_DIMENSION: u32 = 65535;

/// Values that are fixed for the lifetime of a solver, and therefore baked into all shaders as
/// constants
pub struct ShaderConstants {
    pub up_axis: usize,
    pub slip_n: usize,
    pub nr_extended_cells: usize,
}

impl ShaderConstants {
    fn as_wgsl(&self) -> String {
        format!(
            "const INTERIOR_OFFSET: u32 = {}u;\n\
             const UP_AXIS: u32 = {}u;\n\
             const SLIP_N: u32 = {}u;\n\
             const N_EXTENDED_CELLS: u32 = {}u;\n\
             const WG_X: u32 = {}u;\n\
             const WG_Y: u32 = {}u;\n\
             const WG_Z: u32 = {}u;\n\
             const WG_1D: u32 = {}u;\n\
             const WG_PLANE: u32 = {}u;\n\
             const WG_REDUCE: u32 = {}u;\n",
            INTERIOR_OFFSET,
            self.up_axis,
            self.slip_n,
            self.nr_extended_cells,
            WG_3D[0], WG_3D[1], WG_3D[2],
            WG_1D,
            WG_PLANE,
            WG_REDUCE
        )
    }
}

#[derive(Debug, Clone, Copy)]
/// The type of each binding in a kernel's bind group
pub enum Binding {
    Uniform,
    ReadOnly,
    ReadWrite,
}

/// A compute pipeline together with the layout of its (single) bind group
pub struct Kernel {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl Kernel {
    pub fn new(
        context: &GpuContext,
        constants: &ShaderConstants,
        kernel_src: &str,
        entry_point: &str,
        bindings: &[Binding]
    ) -> Self {
        let shader_src = format!(
            "{}\n{}\n{}\n{}",
            GPU_GRID_SRC,
            constants.as_wgsl(),
            COMMON_SRC,
            kernel_src
        );

        let entries: Vec<wgpu::BindGroupLayoutEntry> = bindings.iter().enumerate().map(
            |(index, binding)| match binding {
                Binding::Uniform => gpu_utils::uniform_bind_group_layout_entry(index),
                Binding::ReadOnly => gpu_utils::storage_bind_group_layout_entry(index, true),
                Binding::ReadWrite => gpu_utils::storage_bind_group_layout_entry(index, false),
            }
        ).collect();

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&entries);
        let pipeline = context.create_pipeline(entry_point, &bind_group_layout, &shader);

        Self {
            pipeline,
            bind_group_layout
        }
    }

    pub fn bind_group(&self, context: &GpuContext, buffers: &[&wgpu::Buffer]) -> wgpu::BindGroup {
        context.create_bind_group(buffers, &self.bind_group_layout)
    }

    /// Records a dispatch of this kernel, in its own compute pass
    pub fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        workgroups: [u32; 3]
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
    }
}

/// Workgroup counts for a kernel over the interior cells of a grid with `interior_shape`
pub fn dispatch_interior(interior_shape: [usize; 3]) -> [u32; 3] {
    [
        gpu_utils::workgroup_count(interior_shape[2], WG_3D[0] as usize),
        gpu_utils::workgroup_count(interior_shape[1], WG_3D[1] as usize),
        gpu_utils::workgroup_count(interior_shape[0], WG_3D[2] as usize),
    ]
}

/// Workgroup counts for a kernel over a list of `n` items. Large lists are split over two
/// dispatch dimensions, which the kernels combine again with `linear_index` (see common.wgsl).
pub fn dispatch_1d(n: usize) -> [u32; 3] {
    let nr_workgroups = gpu_utils::workgroup_count(n, WG_1D as usize);

    let x = nr_workgroups.min(MAX_WORKGROUPS_PER_DIMENSION);
    let y = nr_workgroups.div_ceil(x);

    [x, y, 1]
}

/// Workgroup counts for a kernel over a boundary face with the two dimensions `[inner, outer]`
pub fn dispatch_plane(inner_length: usize, outer_length: usize) -> [u32; 3] {
    [
        gpu_utils::workgroup_count(inner_length, WG_PLANE as usize),
        gpu_utils::workgroup_count(outer_length, WG_PLANE as usize),
        1
    ]
}

/// Workgroup counts for the reduction kernel over `n` items
pub fn dispatch_reduce(n: usize) -> [u32; 3] {
    [gpu_utils::workgroup_count(n, WG_REDUCE as usize).min(MAX_REDUCE_WORKGROUPS), 1, 1]
}
