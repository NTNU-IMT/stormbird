
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultigridSettings {
    #[serde(default="MultigridSettings::default_nr_v_cycles")]
    pub nr_v_cycles: usize,
    #[serde(default="MultigridSettings::default_nr_smooth_iterations")]
    pub nr_smooth_iterations: usize
}

impl MultigridSettings {
    pub fn default_nr_v_cycles() -> usize {2}
    pub fn default_nr_smooth_iterations() -> usize {4}
}

impl Default for MultigridSettings {
    fn default() -> Self {
        Self {
            nr_v_cycles: Self::default_nr_v_cycles(),
            nr_smooth_iterations: Self::default_nr_smooth_iterations()
        }
    }
}