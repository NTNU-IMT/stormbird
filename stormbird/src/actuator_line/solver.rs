use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverSettings {
    #[serde(default="SolverSettings::default_damping_factor")]
    pub damping_factor: f64,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self {
            damping_factor: Self::default_damping_factor(),
        }
    }
}

impl SolverSettings {
    fn default_damping_factor() -> f64 {0.5}
}