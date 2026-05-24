use serde::{Serialize, Deserialize};

use crate::boundary_conditions::pressure::PressureBoundaryConditions;
use crate::grid::Grid;

use super::{
    PressureSolverMultiGrid,
    PressureSolverSettings
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressureSolverBuilder {
    pub settings: PressureSolverSettings,
}

impl PressureSolverBuilder {
    pub fn build(&self, grid: &Grid, boundary_conditions: &PressureBoundaryConditions) -> PressureSolverMultiGrid {
        PressureSolverMultiGrid::new(
            grid, 
            boundary_conditions, self.settings.clone()
        )
    }
}
