// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;
use stormath::consts::PI;

use crate::line_force_model::LineForceModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Empirical correction of the circulation close to the ends of the wings, with a shape similar to
/// the Prandtl tip loss factor
pub struct TipLossCorrection {
    /// Factor that determines how fast the correction approaches one away from the ends of the wings
    #[serde(default = "TipLossCorrection::default_exp_factor")]
    pub exp_factor: Float,
    /// If true, the corrected circulation on each wing is scaled so that the mean value over the
    /// span is the same as before the correction
    #[serde(default = "TipLossCorrection::default_maintain_mean_value")]
    pub maintain_mean_value: bool,
}

impl Default for TipLossCorrection {
    fn default() -> Self {
        Self {
            exp_factor: Self::default_exp_factor(),
            maintain_mean_value: Self::default_maintain_mean_value(),
        }
    }
}

impl TipLossCorrection {
    fn default_exp_factor() -> Float {10.0}
    fn default_maintain_mean_value() -> bool {true}

    /// Lower limit for the mean value of the corrected circulation on a wing, relative to the mean
    /// of the absolute value of the uncorrected circulation, for the mean value to be maintained.
    /// Below this limit, the scaling would divide by a value close to zero, and the corrected
    /// circulation is used without scaling.
    const MIN_RELATIVE_CORRECTED_MEAN: Float = 1e-3;

    /// Returns the correction factor at the given non-dimensional span position, running from -0.5
    /// to 0.5: (2 / pi) * acos(exp(-exp_factor * (0.5 - |s|)))
    pub fn correction_factor(&self, non_dim_span_position: Float) -> Float {
        let s_in = 0.5 - non_dim_span_position.abs();

        let exp_value = (-self.exp_factor * s_in).exp();

        (2.0 / PI) * exp_value.acos()
    }

    /// Applies the correction to the circulation strength on all the wings in the line force model.
    ///
    /// If `maintain_mean_value` is true, the corrected circulation on each wing is scaled so that
    /// the mean value over the span, weighted with the length of each line segment, is the same as
    /// before the correction.
    pub fn apply(&self, circulation_strength: &mut [Float], line_force_model: &LineForceModel) {
        let non_dim_span_positions = &line_force_model.ctrl_point_spanwise_distance_circulation_model;

        for wing_indices in &line_force_model.wing_indices {
            let mut raw_integral = 0.0;
            let mut raw_absolute_integral = 0.0;
            let mut corrected_integral = 0.0;

            for i in wing_indices.clone() {
                let length = line_force_model.span_lines_local[i].length();

                raw_integral += circulation_strength[i] * length;
                raw_absolute_integral += circulation_strength[i].abs() * length;

                circulation_strength[i] *= self.correction_factor(non_dim_span_positions[i]);

                corrected_integral += circulation_strength[i] * length;
            }

            let scaling_is_well_defined =
                corrected_integral.abs() > Self::MIN_RELATIVE_CORRECTED_MEAN * raw_absolute_integral;

            if self.maintain_mean_value && scaling_is_well_defined {
                let scale = raw_integral / corrected_integral;

                for i in wing_indices.clone() {
                    circulation_strength[i] *= scale;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use stormath::spatial_vector::SpatialVector;

    use crate::line_force_model::builder::LineForceModelBuilder;
    use crate::line_force_model::builder::single_wing::WingBuilder;
    use crate::section_models::SectionModel;
    use crate::section_models::foil::Foil;

    /// Two straight wings along the z-axis, with different span lengths
    fn two_wings() -> LineForceModel {
        let mut builder = LineForceModelBuilder::new(16);

        for (x, span) in [(0.0, 30.0), (20.0, 15.0)] {
            builder.add_wing(WingBuilder {
                section_points: vec![
                    SpatialVector::from([x, 0.0, 2.0]),
                    SpatialVector::from([x, 0.0, 2.0 + span]),
                ],
                chord_vectors: vec![SpatialVector::from([5.0, 0.0, 0.0]); 2],
                section_model: SectionModel::SingleElementFoil(Foil::default()),
                ..Default::default()
            });
        }

        builder.build()
    }

    fn wing_integrals(circulation_strength: &[Float], line_force_model: &LineForceModel) -> Vec<Float> {
        line_force_model.wing_indices.iter().map(|wing_indices| {
            wing_indices.clone().map(|i| {
                circulation_strength[i] * line_force_model.span_lines_local[i].length()
            }).sum()
        }).collect()
    }

    #[test]
    fn correction_factor_is_zero_at_the_ends() {
        let correction = TipLossCorrection::default();

        assert!(correction.correction_factor(0.5).abs() < 1e-6);
        assert!(correction.correction_factor(-0.5).abs() < 1e-6);
        assert!(correction.correction_factor(0.0) > 0.99);
    }

    #[test]
    fn maintain_mean_value_keeps_the_mean_on_each_wing() {
        let line_force_model = two_wings();
        let nr_span_lines = line_force_model.nr_span_lines();

        let raw_circulation_strength: Vec<Float> = (0..nr_span_lines)
            .map(|i| 10.0 + i as Float)
            .collect();

        let mut corrected = raw_circulation_strength.clone();
        TipLossCorrection::default().apply(&mut corrected, &line_force_model);

        let raw_integrals = wing_integrals(&raw_circulation_strength, &line_force_model);
        let corrected_integrals = wing_integrals(&corrected, &line_force_model);

        for (raw, corrected) in raw_integrals.iter().zip(corrected_integrals.iter()) {
            assert!(((corrected - raw) / raw).abs() < 1e-4);
        }

        // The circulation is still reduced at the ends
        let first_index = line_force_model.wing_indices[0].start;
        assert!(corrected[first_index] < raw_circulation_strength[first_index]);
    }

    #[test]
    fn without_maintain_mean_value_the_mean_is_reduced() {
        let line_force_model = two_wings();
        let nr_span_lines = line_force_model.nr_span_lines();

        let raw_circulation_strength = vec![10.0; nr_span_lines];

        let mut corrected = raw_circulation_strength.clone();
        TipLossCorrection {
            maintain_mean_value: false,
            ..Default::default()
        }.apply(&mut corrected, &line_force_model);

        let raw_integrals = wing_integrals(&raw_circulation_strength, &line_force_model);
        let corrected_integrals = wing_integrals(&corrected, &line_force_model);

        for (raw, corrected) in raw_integrals.iter().zip(corrected_integrals.iter()) {
            assert!(corrected < raw);
        }
    }

    #[test]
    fn no_scaling_when_the_corrected_mean_is_close_to_zero() {
        let line_force_model = two_wings();
        let nr_span_lines = line_force_model.nr_span_lines();

        let raw_circulation_strength = vec![0.0; nr_span_lines];

        let mut corrected = raw_circulation_strength.clone();
        TipLossCorrection::default().apply(&mut corrected, &line_force_model);

        assert!(corrected.iter().all(|value| value.is_finite() && *value == 0.0));
    }
}
