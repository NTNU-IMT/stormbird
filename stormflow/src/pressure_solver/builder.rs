use serde::{Serialize, Deserialize};

use super::boundary_conditions::PressureBoundaryConditions;
use crate::grid::Grid;

use super::{
    PressureSolver,
    multigrid_cpu::MultigridCPU,
    multigrid_gpu::MultigridGPU,
    fft::FftCPU,
    settings::MultigridSettings
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PressureSolverBuilder {
    MultigridCPU(MultigridSettings),
    MultigrdiGPU(MultigridSettings),
    FftCPU
}

impl Default for PressureSolverBuilder {
    fn default() -> Self {
        Self::MultigridCPU(MultigridSettings::default())
    }
}

impl PressureSolverBuilder {
    pub fn build(&self, grid: &Grid, boundary_conditions: &PressureBoundaryConditions) -> PressureSolver {
        match self {
            Self::MultigridCPU(settings) => {
                PressureSolver::MultigridCPU(
                    MultigridCPU::new(
                        grid, 
                        boundary_conditions, settings.clone()
                    )
                )
            },
            Self::MultigrdiGPU(settings) => {
                PressureSolver::MultigridGPU(
                    MultigridGPU::new(
                        grid,
                        boundary_conditions, settings.clone()
                    )
                )
            },
            Self::FftCPU => {
                PressureSolver::FftCPU(
                    FftCPU::new(grid, boundary_conditions)
                )
            }
        }

    }
}
