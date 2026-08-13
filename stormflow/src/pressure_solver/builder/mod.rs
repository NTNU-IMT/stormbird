use serde::{Serialize, Deserialize};

use super::boundary_conditions::PressureBoundaryConditions;
use crate::grid::Grid;
use crate::geometry::Geometry;

pub mod settings;

use settings::{
    MultigridSettingsBuilder,
    ComputePlatform
};

use super::{
    PressureSolver,
    multigrid_cpu::MultigridCPU,
    multigrid_gpu::MultigridGPU,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PressureSolverBuilder {
    Multigrid(MultigridSettingsBuilder),
}

impl Default for PressureSolverBuilder {
    fn default() -> Self {
        Self::Multigrid(MultigridSettingsBuilder::default())
    }
}

impl PressureSolverBuilder {
    pub fn build(
        &self,
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
        slip_geometries: &[Geometry]
    ) -> PressureSolver {
        match self {
            Self::Multigrid(settings) => {
                match settings.compute_platform {
                    ComputePlatform::CPU => {
                        PressureSolver::MultigridCPU(
                            MultigridCPU::new(
                                grid,
                                boundary_conditions,
                                settings.build_settings(),
                                slip_geometries
                            )
                        )
                    },
                    ComputePlatform::GPU => {
                        PressureSolver::MultigridGPU(
                            MultigridGPU::new(
                                grid, 
                                boundary_conditions, 
                                settings.build_settings()
                            )
                        )
                    }
                }
            },
        }

    }
}
