pub mod builder;
pub mod settings;
pub mod cpu_version;
pub mod gpu_version;

use cpu_version::PressureSolverCPU;

pub enum PressureSolver {
    CPU(PressureSolverCPU)
}

impl PressureSolver {
    pub fn solve(&mut self) {
        match self {
            PressureSolver::CPU(solver) => solver.solve()
        }
    }
}