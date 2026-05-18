use stormath::sparse_matrix::linalg::IterativeSolverSettings;

use serde::{Serialize, Deserialize};

use crate::boundary_conditions::BoundaryConditions;
use crate::grid::Grid;

use super::PressureSolver;
use super::jacobi::PressureSolverJacobi;
use super::multigrid::PressureSolverMultiGrid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PressureSolverType {
    #[default]
    Jacobi,
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
            PressureSolverType::Jacobi => {
                let (mut matrix, rhs) = PressureSolver::poisson_matrix_and_rhs(grid, boundary_conditions);

                matrix.set_diagonal_data();
                
                PressureSolver::Jacobi(
                    PressureSolverJacobi {
                        matrix,
                        work: vec![0.0; rhs.len()],
                        solver_settings: self.solver_settings.clone()
                    }
                )
            },
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
