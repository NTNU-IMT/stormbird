// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for the sampling of the velocity at the line elements from the CFD grid
pub struct SamplingSettings {
    /// If true, the velocity is sampled directly at the control points, rather than as a weighted
    /// average over the cells close to each line element
    #[serde(default)]
    pub use_point_sampling: bool,
    /// The smoothing length used for the weights along the span, relative to the length of each line
    /// element
    #[serde(default="SamplingSettings::default_span_projection_factor")]
    pub span_projection_factor: Float,
    /// If true, the weights do not depend on the location along the span
    #[serde(default)]
    pub neglect_span_projection: bool,
    /// Cells with a weight below this value, relative to the peak value of the projection weight
    /// of the dominating line element, are not used in the sampling. See
    /// [`ActuatorLine::relative_projection_weight`](crate::actuator_line::ActuatorLine::relative_projection_weight).
    #[serde(default="SamplingSettings::default_weight_limit")]
    pub weight_limit: Float,
    /// If true, the velocity at the first and last line element of each wing is extrapolated from the
    /// neighboring line elements
    #[serde(default)]
    pub extrapolate_end_velocities: bool,
    /// If true, the velocity component along the span is removed. True by default.
    #[serde(default="SamplingSettings::default_true")]
    pub remove_span_velocity: bool,
    /// Factor multiplied with the sampled velocity
    #[serde(default="SamplingSettings::default_one")]
    pub correction_factor: Float
}

impl SamplingSettings {
    fn default_one() -> Float {1.0}
    fn default_true() -> bool {true}
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
            remove_span_velocity: Self::default_true(),
            correction_factor: Self::default_one(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// The velocity along the span is removed by default, both for the default settings and for
    /// settings read from JSON without the field
    fn the_span_velocity_is_removed_by_default() {
        assert!(SamplingSettings::default().remove_span_velocity);

        let from_json: SamplingSettings = serde_json::from_str("{}").unwrap();

        assert!(from_json.remove_span_velocity);
    }
}
