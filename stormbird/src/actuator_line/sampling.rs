use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingSettings {
    #[serde(default)]
    pub use_point_sampling: bool,
    #[serde(default="SamplingSettings::default_span_projection_factor")]
    pub span_projection_factor: f64,
    #[serde(default)]
    pub neglect_span_projection: bool,
    #[serde(default="SamplingSettings::default_weight_limit")]
    pub weight_limit: f64,
    #[serde(default)]
    pub extrapolate_end_velocities: bool,
    #[serde(default)]
    pub remove_span_velocity: bool,

}

impl SamplingSettings {
    fn default_span_projection_factor() -> f64 {0.5}
    fn default_weight_limit() -> f64 {0.001}
}

impl Default for SamplingSettings {
    fn default() -> Self {
        Self {
            use_point_sampling: false,
            span_projection_factor: Self::default_span_projection_factor(),
            neglect_span_projection: false,
            weight_limit: Self::default_weight_limit(),
            extrapolate_end_velocities: false,
            remove_span_velocity: false,
        }
    }
}