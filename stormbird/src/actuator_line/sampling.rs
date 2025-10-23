// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingSettings {
    #[serde(default)]
    pub use_point_sampling: bool,
    #[serde(default="SamplingSettings::default_span_projection_factor")]
    pub span_projection_factor: Float,
    #[serde(default)]
    pub neglect_span_projection: bool,
    #[serde(default="SamplingSettings::default_weight_limit")]
    pub weight_limit: Float,
    #[serde(default)]
    pub extrapolate_end_velocities: bool,
    #[serde(default)]
    pub remove_span_velocity: bool,
    #[serde(default="SamplingSettings::default_one")]
    pub correction_factor: Float
}

impl SamplingSettings {
    fn default_one() -> Float {1.0}
    fn default_span_projection_factor() -> Float {0.5}
    fn default_weight_limit() -> Float {0.001}
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
            correction_factor: Self::default_one(),
        }
    }
}
