
use crate::gpu_interface::{
    context::GpuContext,
    utils as gpu_utils
};

const GPU_GRID_SRC: &str = include_str!("../../../grid/gpu_version/gpu_grid.wgsl");
const JACOBI_COMMON_SRC: &str = include_str!("jacobi_common.wgsl");
const JACOBI_SLIP_SRC: &str = include_str!("jacobi_slip_shader.wgsl");

use crate::grid::gpu_version::GpuGrid;
use crate::pressure_solver::boundary_conditions::PressureBoundaryConditions;
use crate::pressure_solver::multigrid_cpu::kernels::jacobi::SLIP_CORRECTION_RELAXATION;
use crate::pressure_solver::multigrid_cpu::slip_pressure_stencils::SlipPressureInterpolationOrder;

use super::bc_consts_wgsl;

pub const WORKGROUP_SIZE: u32 = 4;

/// Weights-per-axis for the mirrored image-point gather, matching
/// `SlipPressureInterpolationOrder`: 2 for `Trilinear`, 4 for `Tricubic`. Determines both the
/// `weights` buffer's per-entry stride and the `SLIP_N` const baked into the shader source.
pub fn weights_per_axis(order: SlipPressureInterpolationOrder) -> usize {
    match order {
        SlipPressureInterpolationOrder::Trilinear => 2,
        SlipPressureInterpolationOrder::Tricubic => 4,
    }
}

fn slip_consts_wgsl(order: SlipPressureInterpolationOrder) -> String {
    format!(
        "const SLIP_N: u32 = {slip_n}u;\nconst SLIP_CORRECTION_RELAXATION: f32 = {relaxation};\n",
        slip_n = weights_per_axis(order),
        relaxation = SLIP_CORRECTION_RELAXATION
    )
}

/// Separate pipeline from `JacobiShader`'s (rather than a runtime branch added to it), so the
/// disabled/no-correction path never binds or pays for the 4 extra buffers below — see
/// `jacobi_slip_shader.wgsl`'s doc comment. Only built at all when
/// `MultigridSettings::enable_slip_pressure_correction` is set (see `MultigridGPU::new`), since
/// compiling an unused pipeline has real cost on the GPU, unlike the CPU path's "always build the
/// (cheap) stencils, gate only the dispatch."
pub struct JacobiSlipShader {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl JacobiSlipShader {
    pub fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 8] {
        [
            GpuGrid::bind_group_layout_entry(0),
            gpu_utils::storage_bind_group_layout_entry(1, true),
            gpu_utils::storage_bind_group_layout_entry(2, true),
            gpu_utils::storage_bind_group_layout_entry(3, false),
            gpu_utils::storage_bind_group_layout_entry(4, true),
            gpu_utils::storage_bind_group_layout_entry(5, true),
            gpu_utils::storage_bind_group_layout_entry(6, true),
            gpu_utils::storage_bind_group_layout_entry(7, true),
        ]
    }

    pub fn new(context: &GpuContext, boundary_conditions: &PressureBoundaryConditions, order: SlipPressureInterpolationOrder) -> Self {
        let shader_src = format!(
            "{grid_src}\n{bc_consts}{slip_consts}{jacobi_src}\n{jacobi_common_src}",
            grid_src = GPU_GRID_SRC,
            bc_consts = bc_consts_wgsl(boundary_conditions),
            slip_consts = slip_consts_wgsl(order),
            jacobi_src = JACOBI_SLIP_SRC,
            jacobi_common_src = JACOBI_COMMON_SRC
        );

        let shader = context.create_shader_module(&shader_src);
        let bind_group_layout = context.create_bind_group_layout(&Self::bind_group_layout_entries());
        let pipeline = context.create_pipeline("main", &bind_group_layout, &shader);

        Self {
            pipeline,
            bind_group_layout
        }
    }

    /// Creates the pair of bind groups needed to alternate the Jacobi smoother between the
    /// solution and work buffers of a single grid level, mirroring `JacobiShader::create_bind_groups`
    /// with the 4 extra slip-correction buffers appended.
    #[allow(clippy::too_many_arguments)]
    pub fn create_bind_groups(
        &self,
        context: &GpuContext,
        grid_buffer: &wgpu::Buffer,
        solution_buffer: &wgpu::Buffer,
        rhs_buffer: &wgpu::Buffer,
        work_buffer: &wgpu::Buffer,
        cell_lookup_buffer: &wgpu::Buffer,
        weights_buffer: &wgpu::Buffer,
        base_index_buffer: &wgpu::Buffer,
        mu_buffer: &wgpu::Buffer,
    ) -> (wgpu::BindGroup, wgpu::BindGroup) {
        let bind_group_sol_to_work = context.create_bind_group(
            &[grid_buffer, solution_buffer, rhs_buffer, work_buffer, cell_lookup_buffer, weights_buffer, base_index_buffer, mu_buffer],
            &self.bind_group_layout
        );

        let bind_group_work_to_sol = context.create_bind_group(
            &[grid_buffer, work_buffer, rhs_buffer, solution_buffer, cell_lookup_buffer, weights_buffer, base_index_buffer, mu_buffer],
            &self.bind_group_layout
        );

        (bind_group_sol_to_work, bind_group_work_to_sol)
    }
}
