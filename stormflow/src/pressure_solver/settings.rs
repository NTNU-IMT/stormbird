
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureSolverSettings {
    #[serde(default="PressureSolverSettings::default_nr_v_cycles")]
    pub nr_v_cycles: usize,
    #[serde(default="PressureSolverSettings::default_nr_smooth_iterations")]
    pub nr_smooth_iterations: usize
}

impl PressureSolverSettings {
    pub fn default_nr_v_cycles() -> usize {2}
    pub fn default_nr_smooth_iterations() -> usize {4}
}

impl Default for PressureSolverSettings {
    fn default() -> Self {
        Self {
            nr_v_cycles: Self::default_nr_v_cycles(),
            nr_smooth_iterations: Self::default_nr_smooth_iterations()
        }
    }
}