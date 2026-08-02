pub mod builder;
pub mod boundary_conditions;
pub mod multigrid_cpu;
pub mod multigrid_gpu;

use multigrid_cpu::MultigridCPU;
use multigrid_gpu::MultigridGPU;

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;
use crate::grid::Grid;

use rayon::prelude::*;

pub enum PressureSolver {
    MultigridCPU(MultigridCPU),
    MultigridGPU(MultigridGPU),
}

impl PressureSolver {
    pub fn solve(&mut self) {
        match self {
            PressureSolver::MultigridCPU(solver) => solver.solve(),
            PressureSolver::MultigridGPU(solver) => solver.solve(),
        }
    }

    pub fn pressure_ref(&self) -> &[Float] {
        match self {
            PressureSolver::MultigridCPU(solver) => &solver.solution,
            PressureSolver::MultigridGPU(solver) => &solver.solution,
        }
    }
 
    pub fn calculate_rhs(
        &mut self,
        grid: &Grid,
        velocity_star: &[SpatialVector],
        density: Float,
        time_step: Float
    ) {
        let nr_interior_cells = grid.nr_interior_cells();

        let inv_time_step = 1.0 / time_step;

        // Write straight into the selected solver's rhs buffer instead of a scratch `Vec`
        // copied over afterwards; this only borrows `self.pressure_solver`, which is disjoint
        // from the `self.grid`/`self.velocity_star`/`self.density` fields the closure below reads.
        let data_ptr: usize = match self {
            PressureSolver::MultigridCPU(solver) => solver.rhs_at_levels[0].as_mut_ptr() as usize,
            PressureSolver::MultigridGPU(solver) => solver.rhs.as_mut_ptr() as usize,
        };

        (0..nr_interior_cells)
            .into_par_iter()
            .for_each(|i_flat_interior| {
                let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
                let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
                let i_0 = grid.flat_index_on_extended_grid(extended_indices);

                let mut new_value = 0.0;

                // 4th order accurate divergence: the standard symmetric 4-point staggered
                // derivative (see `add_pressure_gradient_kernel`, which uses the same formula in
                // the opposite staggering direction).
                for axis_index in 0..3 {
                    let stride = grid.extended_stride[axis_index];

                    let i_n = i_0 - stride;
                    let i_p = i_0 + stride;
                    let i_n2 = i_n - stride;

                    new_value += (
                        27.0 * (velocity_star[i_0][axis_index] - velocity_star[i_n][axis_index]) -
                        (velocity_star[i_p][axis_index] - velocity_star[i_n2][axis_index])
                    ) * grid.inv_cell_length[axis_index] * (1.0 / 24.0);
                }

                new_value *= density * inv_time_step;

                unsafe {
                    let ptr = data_ptr as *mut Float;
                    *ptr.add(i_flat_interior) = new_value;
                }
                
            });
    }
}