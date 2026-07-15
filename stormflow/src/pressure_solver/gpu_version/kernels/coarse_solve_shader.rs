
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

use crate::grid::Grid;
use crate::grid::gpu_version::GpuGrid;
use crate::boundary_conditions::pressure::PressureBoundaryConditions;

use super::bc_consts_wgsl;

const GRID_SRC: &str = include_str!("../../../grid/gpu_version/grid.wgsl");
const COARSE_SOLVE_SRC: &str = include_str!("coarse_solve_shader.wgsl");

/// Runs an entire bottom-of-V-cycle smoother (all Jacobi sweeps) for a single, small-enough
/// coarsest grid level in one dispatch, using workgroup-shared memory instead of one dispatch per
/// iteration. With no ghost cells, every thread is a plain interior cell running the same
/// boundary-folded stencil as `JacobiShader`, so there's no per-cell classification to precompute.
pub struct CoarseSolveShader {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
}

impl CoarseSolveShader {
    fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 3] {
        [
            GpuGrid::bind_group_layout_entry(0),
            gpu_utils::storage_bind_group_layout_entry(1, true),  // rhs
            gpu_utils::storage_bind_group_layout_entry(2, false), // x
        ]
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
        let n = grid.nr_interior_cells() as u32;

        let limits = context.device.limits();
        if n > limits.max_compute_invocations_per_workgroup || 
            n > limits.max_compute_workgroup_size_x {
            return None;
        }

        let shader_src = format!(
            "{grid_src}\nconst N: u32 = {n}u;\nconst NR_ITERATIONS: u32 = {nr_iterations}u;\n{bc_consts}{coarse_solve_src}",
            grid_src = GRID_SRC,
            n = n,
            nr_iterations = nr_iterations,
            bc_consts = bc_consts_wgsl(boundary_conditions),
            coarse_solve_src = COARSE_SOLVE_SRC
        );

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("main", &bind_group_layout, &shader);

        let bind_group = context.create_bind_group(
            &[grid_buffer, rhs_buffer, x_buffer],
            &bind_group_layout
        );

        Some(Self { pipeline, bind_group })
    }

    pub fn dispatch(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
