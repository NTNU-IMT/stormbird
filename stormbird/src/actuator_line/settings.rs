use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverSettings {
    #[serde(default)]
    pub strength_damping: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingSettings {
    #[serde(default="SamplingSettings::default_span_projection_factor")]
    pub span_projection_factor: f64,
    #[serde(default)]
    pub neglect_span_projection: bool
}

impl SamplingSettings {
    fn default_span_projection_factor() -> f64 {0.5}
}

impl Default for SamplingSettings {
    fn default() -> Self {
        Self {
            span_projection_factor: Self::default_span_projection_factor(),
            neglect_span_projection: false
        }
    }
}