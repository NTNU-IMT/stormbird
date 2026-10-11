pub mod builder;
pub mod boundary_conditions;
pub mod multigrid_cpu;
pub mod multigrid_gpu;

use multigrid_cpu::MultigridCPU;
use multigrid_gpu::MultigridGPU;

use std::borrow::Cow;

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

    /// The pressure on the host. Note that this is not updated for the GPU version when it is
    /// solved with `MultigridGPU::solve_on_device`. See `pressure_host` for a version that always
    /// returns the current values.
    pub fn pressure_ref(&self) -> &[Float] {
        match self {
            PressureSolver::MultigridCPU(solver) => &solver.solution,
            PressureSolver::MultigridGPU(solver) => &solver.solution,
        }
    }
 
    /// The pressure on the host, for output purposes. Borrowed for the CPU version, and read from
    /// the device for the GPU version, which is always up to date on the device, also when it is
    /// solved with `MultigridGPU::solve_on_device`.
    pub fn pressure_host(&self) -> Cow<'_, [Float]> {
        match self {
            PressureSolver::MultigridCPU(solver) => Cow::Borrowed(&solver.solution),
            PressureSolver::MultigridGPU(solver) => Cow::Owned(solver.read_solution()),
        }
    }
}
