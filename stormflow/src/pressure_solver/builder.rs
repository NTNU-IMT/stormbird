use stormath::sparse_matrix::linalg::IterativeSolverSettings;

use serde::{Serialize, Deserialize};

use crate::boundary_conditions::BoundaryConditions;
use crate::grid::Grid;

use super::PressureSolver;
use super::multigrid::PressureSolverMultiGrid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PressureSolverType {
    #[default]
    Multigrid
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressureSolverBuilder {
    pub solver_settings: IterativeSolverSettings,
    pub solver_type: PressureSolverType   
}

impl PressureSolverBuilder {
    pub fn build(&self, grid: &Grid, boundary_conditions: &BoundaryConditions) -> PressureSolver {
        match self.solver_type {
            PressureSolverType::Multigrid => {
                PressureSolver::Multigrid(
                    PressureSolverMultiGrid::new(
                        grid, 
                        boundary_conditions, self.solver_settings.clone()
                    )
                )
            }
        }
    }
}
