pub mod builder;
pub mod boundary_conditions;
pub mod multigrid_cpu;
pub mod multigrid_gpu;

use multigrid_cpu::MultigridCPU;
use multigrid_gpu::MultigridGPU;

use stormath::type_aliases::Float;

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
 
    /// The right hand side of the pressure Poisson equation on the finest level, stored on the 
    /// **interior** grid. Must be populated before calling `solve`.
    pub fn rhs_mut(&mut self) -> &mut [Float] {
        match self {
            PressureSolver::MultigridCPU(solver) => &mut solver.rhs_at_levels[0],
            PressureSolver::MultigridGPU(solver) => &mut solver.rhs,
        }
    }
}