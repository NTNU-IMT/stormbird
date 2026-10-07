use serde::{Serialize, Deserialize};

use super::boundary_conditions::PressureBoundaryConditions;
use crate::grid::Grid;
use crate::geometry::Geometry;
use crate::gpu_interface::context::GpuContext;

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
    pub fn compute_platform(&self) -> ComputePlatform {
        match self {
            Self::Multigrid(settings) => settings.compute_platform,
        }
    }

    /// Builds the solver. A GPU solver is created on the device in `gpu_context` if given, so that
    /// it can share buffers with other solvers, and on a new device otherwise.
    pub fn build(
        &self,
        grid: &Grid,
        boundary_conditions: &PressureBoundaryConditions,
        slip_geometries: &[Geometry],
        gpu_context: Option<&GpuContext>
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
                            MultigridGPU::new_with_context(
                                gpu_context.cloned().unwrap_or_default(),
                                grid,
                                boundary_conditions,
                                settings.build_settings(),
                                slip_geometries
                            )
                        )
                    }
                }
            },
        }

    }
}
