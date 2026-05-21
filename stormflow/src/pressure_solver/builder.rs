use stormath::sparse_matrix::linalg::IterativeSolverSettings;

use serde::{Serialize, Deserialize};

use crate::boundary_conditions::BoundaryConditions;
use crate::grid::Grid;

use super::PressureSolverMultiGrid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressureSolverBuilder {
    pub settings: IterativeSolverSettings,
}

impl PressureSolverBuilder {
    pub fn build(&self, grid: &Grid, boundary_conditions: &BoundaryConditions) -> PressureSolverMultiGrid {
        PressureSolverMultiGrid::new(
            grid, 
            boundary_conditions, self.settings.clone()
        )
    }
}
