pub mod builder;
pub mod settings;
pub mod boundary_conditions;
pub mod multigrid_cpu;
pub mod multigrid_gpu;
pub mod fft;

use multigrid_cpu::MultigridCPU;
use multigrid_gpu::MultigridGPU;
use fft::FftCPU;

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
}