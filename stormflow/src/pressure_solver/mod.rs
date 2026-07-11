pub mod builder;
pub mod settings;
pub mod cpu_version;
pub mod gpu_version;

use cpu_version::PressureSolverCPU;
use gpu_version::PressureSolverGPU;

pub enum PressureSolver {
    CPU(PressureSolverCPU),
    GPU(PressureSolverGPU)
}

impl PressureSolver {
    pub fn solve(&mut self) {
        match self {
            PressureSolver::CPU(solver) => solver.solve(),
            PressureSolver::GPU(solver) => solver.solve()
        }
    }
}