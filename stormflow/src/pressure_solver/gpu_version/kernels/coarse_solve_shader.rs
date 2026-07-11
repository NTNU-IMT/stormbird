
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use crate::grid::Grid;
use crate::grid::gpu_version::GpuGrid;
use crate::boundary_conditions::pressure::PressureBoundaryConditions;

const GRID_SRC: &str = include_str!("../../../grid/gpu_version/grid.wgsl");
const COARSE_SOLVE_SRC: &str = include_str!("coarse_solve_shader.wgsl");

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CellDescriptor {
    kind: u32,
    neighbor_delta: i32,
    zero_value: u32,
    rhs_index: u32,
}

const KIND_UNUSED: u32 = 0;
const KIND_INTERIOR: u32 = 1;
const KIND_GHOST: u32 = 2;

/// Runs an entire bottom-of-V-cycle smoother (all Jacobi + ghost-cell sweeps) for a single,
/// small-enough coarsest grid level in one dispatch, using workgroup-shared memory instead of
/// one dispatch per iteration.
pub struct CoarseSolveShader {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    cells_buffer: wgpu::Buffer,
}

impl CoarseSolveShader {
    fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 4] {
        [
            GpuGrid::bind_group_layout_entry(0),
            gpu_utils::storage_bind_group_layout_entry(1, true),  // cells
            gpu_utils::storage_bind_group_layout_entry(2, true),  // rhs
            gpu_utils::storage_bind_group_layout_entry(3, false), // x
        ]
    }

    /// Classifies every extended-grid cell of `grid` as unused (corner/edge, never read by any
    /// stencil in this solver), interior, or core-ghost, precomputing whatever each kind needs
    /// (RHS index for interior cells, neighbor offset + BC flag for ghost cells). Doing this on
    /// the CPU keeps the axis/face logic in ordinary, easily-checked Rust instead of re-deriving
    /// it per-thread in WGSL.
    fn build_cell_descriptors(grid: &Grid, boundary_conditions: &PressureBoundaryConditions) -> Vec<CellDescriptor> {
        let n = grid.nr_extended_cells();
        let mut descriptors = vec![CellDescriptor { kind: KIND_UNUSED, neighbor_delta: 0, zero_value: 0, rhs_index: 0 }; n];

        let [nx, ny, nz] = grid.interior_shape;

        for flat in 0..n {
            let [i, j, k] = grid.extended_indices_from_flat_index(flat);

            let x_ghost = i == 0 || i == nx + 1;
            let y_ghost = j == 0 || j == ny + 1;
            let z_ghost = k == 0 || k == nz + 1;

            let ghost_count = [x_ghost, y_ghost, z_ghost].iter().filter(|&&b| b).count();

            if ghost_count == 0 {
                let rhs_index = grid.flat_index_on_interior_grid([i - 1, j - 1, k - 1]) as u32;
                descriptors[flat] = CellDescriptor { kind: KIND_INTERIOR, neighbor_delta: 0, zero_value: 0, rhs_index };
            } else if ghost_count == 1 {
                let (axis_index, face_index) = if x_ghost {
                    (0, if i == 0 { 0 } else { 1 })
                } else if y_ghost {
                    (1, if j == 0 { 0 } else { 1 })
                } else {
                    (2, if k == 0 { 0 } else { 1 })
                };

                let axis_stride = grid.extended_stride[axis_index] as i32;
                let neighbor_delta = if face_index == 0 { axis_stride } else { -axis_stride };
                let zero_value = boundary_conditions.condition(axis_index, face_index).as_gpu_flag();

                descriptors[flat] = CellDescriptor { kind: KIND_GHOST, neighbor_delta, zero_value, rhs_index: 0 };
            }
            // ghost_count >= 2: corner/edge cell, never read downstream — leave as KIND_UNUSED.
        }

        descriptors
    }

    /// Returns `None` if the coarsest grid doesn't fit in a single workgroup on this device, in
    /// which case the caller should fall back to the regular per-iteration dispatch path.
    pub fn try_new(
        context: &GpuContext,
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
        nr_iterations: usize,
        grid_buffer: &wgpu::Buffer,
        rhs_buffer: &wgpu::Buffer,
        x_buffer: &wgpu::Buffer,
    ) -> Option<Self> {
        let n = grid.nr_extended_cells() as u32;

        let limits = context.device.limits();
        if n > limits.max_compute_invocations_per_workgroup || n > limits.max_compute_workgroup_size_x {
            return None;
        }

        let descriptors = Self::build_cell_descriptors(grid, boundary_conditions);
        let cells_buffer = context.create_storage_buffer_init(&descriptors);

        let shader_src = format!(
            "{grid_src}\nconst N: u32 = {n}u;\nconst NR_ITERATIONS: u32 = {nr_iterations}u;\n{coarse_solve_src}",
            grid_src = GRID_SRC,
            n = n,
            nr_iterations = nr_iterations,
            coarse_solve_src = COARSE_SOLVE_SRC
        );

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("main", &bind_group_layout, &shader);

        let bind_group = context.create_bind_group(
            &[grid_buffer, &cells_buffer, rhs_buffer, x_buffer],
            &bind_group_layout
        );

        Some(Self { pipeline, bind_group, cells_buffer })
    }

    pub fn dispatch(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
