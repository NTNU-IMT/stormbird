use stormath::type_aliases::Float;

use crate::pressure_solver::multigrid_cpu::slip_pressure_stencils::{
    SlipPressureStencils, SlipPressureInterpolationStencil
};

/// Flattens a level's `SlipPressureStencils` into the three flat buffers `jacobi_slip_shader.wgsl`
/// expects (`weights`, `base_index`, `mu` — `cell_lookup` needs no conversion and uploads directly
/// via `GpuContext::create_storage_buffer_init`). The per-entry weights stride (`SLIP_N * 3`) falls
/// out naturally from whichever `SlipPressureInterpolationStencil` variant the entries were built
/// with — every entry in a given `SlipPressureStencils` shares the same variant, since
/// `SlipPressureInterpolationOrder` is chosen once for the whole solver, not per entry.
pub fn gpu_buffers_from_stencils(stencils: &SlipPressureStencils) -> (Vec<Float>, Vec<u32>, Vec<Float>) {
    let mut weights = Vec::new();
    let mut base_index = Vec::with_capacity(stencils.entries.len());
    let mut mu = Vec::with_capacity(stencils.entries.len());

    for entry in &stencils.entries {
        mu.push(entry.mu);

        match &entry.stencil {
            SlipPressureInterpolationStencil::Trilinear(stencil) => {
                base_index.push(stencil.base_index as u32);

                for axis_weights in &stencil.weights {
                    weights.extend_from_slice(axis_weights);
                }
            },
            SlipPressureInterpolationStencil::Tricubic(stencil) => {
                base_index.push(stencil.base_index as u32);

                for axis_weights in &stencil.weights {
                    weights.extend_from_slice(axis_weights);
                }
            },
        }
    }

    (weights, base_index, mu)
}
