
use serde::{Serialize, Deserialize};

use crate::pressure_solver::multigrid_cpu::settings::{MultigridSettings, CoarsestLevelSolver};
use crate::pressure_solver::multigrid_cpu::slip_pressure_stencils::SlipPressureInterpolationOrder;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ComputePlatform {
    #[default]
    CPU,
    GPU
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultigridSettingsBuilder {
    #[serde(default="MultigridSettingsBuilder::default_nr_v_cycles")]
    pub nr_v_cycles: usize,
    #[serde(default="MultigridSettingsBuilder::default_nr_smooth_iterations")]
    pub nr_smooth_iterations: usize,
    #[serde(default)]
    pub compute_residual_after_solve: bool,
    #[serde(default)]
    pub compute_platform: ComputePlatform,
    #[serde(default)]
    pub coarsest_level_solver: CoarsestLevelSolver,
    /// Experimental: applies a pressure zero-gradient (Neumann) correction near slip walls during
    /// each V-cycle. Off by default; only has an effect on `ComputePlatform::CPU`.
    #[serde(default)]
    pub enable_slip_pressure_correction: bool,
    /// Interpolation order for the slip-wall pressure correction specifically (ignored when
    /// `enable_slip_pressure_correction` is false) — does not affect the rest of the pressure
    /// solve, which always uses 4th order stencils.
    #[serde(default)]
    pub slip_pressure_interpolation_order: SlipPressureInterpolationOrder
}

impl MultigridSettingsBuilder {
    pub fn default_nr_v_cycles() -> usize {2}
    pub fn default_nr_smooth_iterations() -> usize {4}

    pub fn build_settings(&self) -> MultigridSettings {
        MultigridSettings{
            nr_v_cycles: self.nr_v_cycles,
            nr_smooth_iterations: self.nr_smooth_iterations,
            compute_residual_after_solve: self.compute_residual_after_solve,
            coarsest_level_solver: self.coarsest_level_solver,
            enable_slip_pressure_correction: self.enable_slip_pressure_correction,
            slip_pressure_interpolation_order: self.slip_pressure_interpolation_order
        }
    }
}

impl Default for MultigridSettingsBuilder {
    fn default() -> Self {
        Self {
            nr_v_cycles: Self::default_nr_v_cycles(),
            nr_smooth_iterations: Self::default_nr_smooth_iterations(),
            compute_residual_after_solve: false,
            compute_platform: ComputePlatform::default(),
            coarsest_level_solver: CoarsestLevelSolver::default(),
            enable_slip_pressure_correction: false,
            slip_pressure_interpolation_order: SlipPressureInterpolationOrder::default()
        }
    }
}