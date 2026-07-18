
use serde::{Serialize, Deserialize};

use crate::pressure_solver::multigrid_cpu::settings::MultigridSettings;

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
    pub compute_platform: ComputePlatform
}

impl MultigridSettingsBuilder {
    pub fn default_nr_v_cycles() -> usize {2}
    pub fn default_nr_smooth_iterations() -> usize {4}

    pub fn build_settings(&self) -> MultigridSettings {
        MultigridSettings{
            nr_v_cycles: self.nr_v_cycles,
            nr_smooth_iterations: self.nr_smooth_iterations
        }
    }
}

impl Default for MultigridSettingsBuilder {
    fn default() -> Self {
        Self {
            nr_v_cycles: Self::default_nr_v_cycles(),
            nr_smooth_iterations: Self::default_nr_smooth_iterations(),
            compute_platform: ComputePlatform::default()
        }
    }
}