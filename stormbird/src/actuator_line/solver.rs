use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverSettings {
    #[serde(default="SolverSettings::default_damping_factor")]
    pub damping_factor: Float,
}

impl Default for SolverSettings {
    fn default() -> Self {
        Self {
            damping_factor: Self::default_damping_factor(),
        }
    }
}

impl SolverSettings {
    fn default_damping_factor() -> Float {0.5}
}