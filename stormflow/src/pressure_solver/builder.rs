use serde::{Serialize, Deserialize};

use crate::boundary_conditions::pressure::PressureBoundaryConditions;
use crate::grid::Grid;

use super::{
    PressureSolver,
    cpu_version::PressureSolverCPU,
    settings::PressureSolverSettings
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum SolverPlatform {
    #[default]
    CPU,
    GPU
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressureSolverBuilder {
    pub settings: PressureSolverSettings,
    pub solver_platform: SolverPlatform
}

impl PressureSolverBuilder {
    pub fn build(&self, grid: &Grid, boundary_conditions: &PressureBoundaryConditions) -> PressureSolver {
        match self.solver_platform {
            SolverPlatform::CPU => {
                PressureSolver::CPU(
                    PressureSolverCPU::new(
                        grid, 
                        boundary_conditions, self.settings.clone()
                    )
                )
            },
            SolverPlatform::GPU => {
                todo!()
            }
        }
        
    }
}
