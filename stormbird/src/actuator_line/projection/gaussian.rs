// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

use crate::line_force_model::span_line::SpanLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gaussian {
    pub chord_factor: Float,
    pub thickness_factor: Float,
    #[serde(default = "Gaussian::default_span_falloff_factor")]
    pub span_falloff_factor: Float,
}

impl Default for Gaussian {
    fn default() -> Self {
        Self {
            chord_factor: 0.4,
            thickness_factor: 0.4,
            span_falloff_factor: Self::default_span_falloff_factor(),
        }
    }
}

impl Gaussian {
    fn default_span_falloff_factor() -> Float {0.25}

    /// Computes the projection value at a given point in space.
    ///
    /// `extended_span_falloff` controls whether the constant span profile is replaced by a
    /// smooth Gaussian falloff beyond the span line's extent, as `[falloff_before_start,
    /// falloff_after_end]`. Enabling either (or both) keeps the total volume integral equal to
    /// one by re-normalizing the plateau/falloff amplitude accordingly.
    pub fn projection_value_at_point(
        &self,
        point: SpatialVector,
        chord_vector: SpatialVector,
        span_line: &SpanLine,
        extended_span_falloff: [bool; 2],
    ) -> Float {
        let chord_length = chord_vector.length();
        let line_length = span_line.length();

        let point_lc = span_line.line_coordinates(point, chord_vector);

        let e_chord     = self.chord_factor * chord_length;
        let e_thickness = self.thickness_factor * chord_length;
        let e_span      = self.span_falloff_factor * line_length;

        let chord_term     = (point_lc.chord / e_chord).powi(2);
        let thickness_term = (point_lc.thickness / e_thickness).powi(2);

        let exp_factor = -chord_term - thickness_term;

        let const_denominator = e_chord * e_thickness * PI;

        let span_falloff_enabled = self.span_falloff_factor > 0.0;

        let [falloff_before_start, falloff_after_end] = extended_span_falloff
            .map(|falloff| falloff && span_falloff_enabled);

        // Half-Gaussian tail area, used so that the total span integral stays equal to one
        // regardless of how many (if any) of the two falloff regions are enabled.
        let half_gaussian_area = e_span * PI.sqrt() / 2.0;

        let span_normalization = line_length
            + if falloff_before_start {half_gaussian_area} else {0.0}
            + if falloff_after_end   {half_gaussian_area} else {0.0};

        let span = point_lc.span;
        let half_length = 0.5 * line_length;

        let span_factor = if span > -half_length && span <= half_length {
            1.0 / span_normalization
        } else if span < -half_length && falloff_before_start {
            let falloff_term = ((span + half_length) / e_span).powi(2);
            (1.0 / span_normalization) * (-falloff_term).exp()
        } else if span > half_length && falloff_after_end {
            let falloff_term = ((span - half_length) / e_span).powi(2);
            (1.0 / span_normalization) * (-falloff_term).exp()
        } else {
            0.0
        };

        (1.0 / const_denominator ) * exp_factor.exp() * span_factor
    }
}
