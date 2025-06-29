use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverSettings {
    #[serde(default)]
    pub strength_damping: f64,
}