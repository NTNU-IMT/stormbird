pub mod builder;
pub mod boundary_conditions;
pub mod multigrid_cpu;
pub mod multigrid_gpu;
pub mod fft;

use multigrid_cpu::MultigridCPU;
use multigrid_gpu::MultigridGPU;
use fft::FftCPU;

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;
use crate::grid::Grid;

use rayon::prelude::*;

pub enum PressureSolver {
    MultigridCPU(MultigridCPU),
    MultigridGPU(MultigridGPU),
    FftCPU(FftCPU)
}

impl PressureSolver {
    pub fn solve(&mut self) {
        match self {
            PressureSolver::MultigridCPU(solver) => solver.solve(),
            PressureSolver::MultigridGPU(solver) => solver.solve(),
            PressureSolver::FftCPU(solver) => solver.solve()
        }
    }

    pub fn pressure_ref(&self) -> &[Float] {
        match self {
            PressureSolver::MultigridCPU(solver) => &solver.solution,
            PressureSolver::MultigridGPU(solver) => &solver.solution,
            PressureSolver::FftCPU(solver) => &solver.solution
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
            PressureSolver::FftCPU(solver) => solver.rhs.as_mut_ptr() as usize,
        };

        (0..nr_interior_cells)
            .into_par_iter()
            .for_each(|i_flat_interior| {
                let interior_indices = grid.interior_indices_from_flat_index(i_flat_interior);
                let extended_indices = grid.extended_indices_from_interior_indices(interior_indices);
                let i_0 = grid.flat_index_on_extended_grid(extended_indices);

                let mut new_value = 0.0;
                
                for axis_index in 0..3 {
                    let mut extended_indices_n = extended_indices;

                    extended_indices_n[axis_index] -= 1;
                    
                    let i_n = grid.flat_index_on_extended_grid(extended_indices_n);

                    new_value += (
                        velocity_star[i_0][axis_index] - 
                        velocity_star[i_n][axis_index]
                    ) * grid.inv_cell_length[axis_index];
                }

                new_value *= density * inv_time_step;

                unsafe {
                    let ptr = data_ptr as *mut Float;
                    *ptr.add(i_flat_interior) = new_value;
                }
                
            });
    }
}