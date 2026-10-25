// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::wake::frozen_wake::FrozenWake;
use crate::lifting_line::singularity_elements::horseshoe_vortex::{
    HorseshoeVortex, ELBOW_VORTEX_LENGTH_FACTOR
};
use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;

use crate::line_force_model::LineForceModel;
use crate::line_force_model::span_line::SpanLine;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::transition_functions::cosine_transition_zero_to_one;

use serde::{Serialize, Deserialize};

/// A builder for the [LiftingLineCorrection] struct. See the documentation for
/// [LiftingLineCorrection] for more information on how it works.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiftingLineCorrectionBuilder {
    /// The length of the wake, relative to the average chord length
    #[serde(default = "LiftingLineCorrectionBuilder::default_wake_length_factor")]
    pub wake_length_factor: Float,
    /// Optional symmetry condition
    #[serde(default)]
    pub symmetry_condition: SymmetryCondition,
    /// Optional time over which the correction is gradually increased from zero, to avoid large
    /// corrections at the start of a simulation
    #[serde(default)]
    pub initialization_time: Option<Float>,
    /// The viscous core length of the projection width divided by this factor gives the small
    /// reference viscous core length
    #[serde(default="LiftingLineCorrectionBuilder::default_reference_core_length_factor")]
    pub reference_core_length_factor: Float,
    /// If true, the induced velocities from all wings are included in the correction
    #[serde(default)]
    pub include_wing_interactions: bool,
    /// Optional limit on the magnitude of the velocity correction at each control point, given as
    /// a ratio of the magnitude of the sampled velocity at the same point. No limit by default.
    #[serde(default)]
    pub max_correction_ratio: Option<Float>,
    /// Length of a virtual extension of each wing, at the first and the last end, relative to the
    /// chord length at that end. The extensions are only used in the reference model with the small
    /// viscous core, and carry the circulation of the outermost line element. They move the tip
    /// vortices away from the ends of the wing, which can be used to model end plates. Zero by
    /// default, which means no extensions.
    #[serde(default)]
    pub virtual_extension_factors: [Float; 2],
}

impl Default for LiftingLineCorrectionBuilder {
    fn default() -> Self {
        Self {
            wake_length_factor: Self::default_wake_length_factor(),
            symmetry_condition: SymmetryCondition::NoSymmetry,
            initialization_time: None,
            reference_core_length_factor: Self::default_reference_core_length_factor(),
            include_wing_interactions: false,
            max_correction_ratio: None,
            virtual_extension_factors: [0.0, 0.0],
        }
    }
}

impl LiftingLineCorrectionBuilder {
    fn default_wake_length_factor() -> Float {100.0}
    fn default_reference_core_length_factor() -> Float {100.0}

    /// Builds the correction, where the viscous core length is the given factor times the average
    /// chord length
    pub fn build(
        &self,
        viscous_core_length_factor: Float,
        line_force_model: &LineForceModel
    ) -> LiftingLineCorrection {
        let chord_lengths: Vec<Float> = line_force_model.chord_lengths.clone();

        let average_chord_length = chord_lengths.iter().sum::<Float>() / chord_lengths.len() as Float;

        let viscous_core_length = viscous_core_length_factor * average_chord_length;

        LiftingLineCorrection {
            viscous_core_length,
            viscous_core_length_reference: viscous_core_length / self.reference_core_length_factor,
            wake_length_factor: self.wake_length_factor,
            symmetry_condition: self.symmetry_condition,
            initialization_time: self.initialization_time,
            include_wing_interactions: self.include_wing_interactions,
            max_correction_ratio: self.max_correction_ratio,
            virtual_extension_factors: self.virtual_extension_factors,
        }
    }
}

#[derive(Debug, Clone)]
/// A structure used to compute corrections for the velocity, based on a lifting line model. The
/// model computes two versions of induced velocities using a lifting line representation; one where
/// the viscous core length is set to a small value, and one where it is set to a value equal to the
/// force projection width. The difference in induced velocities are then added to the velocity that
/// is sampled directly from the flow field. This is to account for the fact that realistic force
/// projection widths tend to smear out the tip vortices, leading to under predicted induced
/// velocities.
///
/// By default, the correction is computed for each wing in isolation, so that only errors in the
/// self-induced velocities are corrected. If `include_wing_interactions` is true, the induced
/// velocities from all wings are included, so that errors in the wing-to-wing interactions are
/// also corrected.
pub struct LiftingLineCorrection {
    /// Viscous core length equal to the force projection width
    pub viscous_core_length: Float,
    /// Small reference viscous core length
    pub viscous_core_length_reference: Float,
    /// The length of the wake, relative to the average chord length
    pub wake_length_factor: Float,
    /// Optional symmetry condition
    pub symmetry_condition: SymmetryCondition,
    /// Optional time over which the correction is gradually increased from zero
    pub initialization_time: Option<Float>,
    /// If true, the induced velocities from all wings are included in the correction
    pub include_wing_interactions: bool,
    /// Optional limit on the magnitude of the velocity correction, as a ratio of the magnitude of
    /// the sampled velocity
    pub max_correction_ratio: Option<Float>,
    /// Length of the virtual extensions of each wing in the reference model, at the first and the
    /// last end, relative to the chord length at that end
    pub virtual_extension_factors: [Float; 2],
}

impl LiftingLineCorrection {
    /// Computes a difference in the induced velocities between two lifting line simulations; one
    /// with a viscous core length equal to the force projection width, and one with a small viscous
    /// core length. The difference is then returned as a vector of induced velocities at the control
    /// points of the line force model.
    ///
    /// Whether each wing is treated in isolation, or whether the interactions between wings are
    /// included, is determined by the `include_wing_interactions` field.
    pub fn velocity_correction(
        &self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        circulation_strength: &[Float],
        time: Float,
    ) -> Vec<SpatialVector> {
        let mut u_i_correction = if self.include_wing_interactions {
            self.velocity_correction_all_wings(
                line_force_model,
                ctrl_points_velocity,
                circulation_strength
            )
        } else {
            self.velocity_correction_isolated_wings(
                line_force_model,
                ctrl_points_velocity,
                circulation_strength
            )
        };

        if let Some(initialization_time) = self.initialization_time {
            let correction_factor = cosine_transition_zero_to_one(time, 0.0, initialization_time);

            for u_i in u_i_correction.iter_mut() {
                *u_i *= correction_factor;
            }
        }

        // The correction can become larger than the sampled velocity close to the ends of the
        // wings, where the reference model with the small viscous core is close to singular. The
        // limit reduces the magnitude of the correction, but keeps its direction.
        if let Some(max_correction_ratio) = self.max_correction_ratio {
            for (u_i, velocity) in u_i_correction.iter_mut().zip(ctrl_points_velocity) {
                let max_magnitude = max_correction_ratio * velocity.length();
                let magnitude = u_i.length();

                if magnitude > max_magnitude {
                    *u_i *= max_magnitude / magnitude;
                }
            }
        }

        u_i_correction
    }

    /// Computes the velocity correction for each wing in isolation. That is, only the velocities
    /// induced by a wing on itself are included, so that only errors in the self-induced velocities
    /// are corrected.
    pub fn velocity_correction_isolated_wings(
        &self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        let span_lines = &line_force_model.span_lines_global;
        let chord_lengths = &line_force_model.chord_lengths;

        let mut u_i_correction: Vec<SpatialVector> = Vec::with_capacity(span_lines.len());

        for wing_index in 0..line_force_model.nr_wings() {
            let wind_indices = line_force_model.wing_indices[wing_index].clone();

            let wing_span_lines = &span_lines[
                wind_indices.clone()
            ];

            let wing_chord_lengths = &chord_lengths[
                wind_indices.clone()
            ];

            let wing_circulation_strength = &circulation_strength[
                wind_indices.clone()
            ];

            let nr_span_lines = wing_span_lines.len();

            let wing_ctrl_points_velocity = &ctrl_points_velocity[
                wind_indices.clone()
            ];

            let mut wing_span_point_velocities = Vec::with_capacity(nr_span_lines + 1);

            wing_span_point_velocities.push(wing_ctrl_points_velocity[0]);

            for i in 0..nr_span_lines-1 {
                wing_span_point_velocities.push(
                    0.5 * (wing_ctrl_points_velocity[i] + wing_ctrl_points_velocity[i+1])
                )
            }

            wing_span_point_velocities.push(wing_ctrl_points_velocity[nr_span_lines-1]);

            let average_chord_length = line_force_model.chord_lengths.iter().sum::<Float>() /
                line_force_model.chord_lengths.len() as Float;

            let wake_length = average_chord_length * self.wake_length_factor;


            let mut frozen_wake_viscous = FrozenWake::new_for_single_wing_from_span_lines_and_velocities(
                wing_span_lines,
                wing_chord_lengths,
                &wing_span_point_velocities,
                wake_length,
                self.viscous_core_length,
                self.symmetry_condition
            );

            let mut frozen_wake_default = FrozenWake::new_for_single_wing_from_span_lines_and_velocities(
                wing_span_lines,
                wing_chord_lengths,
                &wing_span_point_velocities,
                wake_length,
                self.viscous_core_length_reference,
                self.symmetry_condition
            );

            frozen_wake_viscous.update_induced_velocities_at_control_points(
                wing_circulation_strength
            );

            frozen_wake_default.update_induced_velocities_at_control_points(
                wing_circulation_strength
            );

            let extension_vortices = self.extension_vortices(
                line_force_model,
                wing_index,
                [wing_span_point_velocities[0], wing_span_point_velocities[nr_span_lines]],
                wake_length
            );

            for i in 0..nr_span_lines {
                let u_i_extensions = self.extension_induced_velocity(
                    &extension_vortices,
                    circulation_strength,
                    wing_span_lines[i].ctrl_point()
                );

                u_i_correction.push(
                    frozen_wake_default.induced_velocities_at_control_points[i] +
                    u_i_extensions -
                    frozen_wake_viscous.induced_velocities_at_control_points[i]
                );
            }
        }

        u_i_correction
    }

    /// Computes the velocity correction with all wings included. That is, the induced velocities
    /// at the control points of each wing are computed from the wakes of all wings, so that errors
    /// in the wing-to-wing interactions are also corrected.
    pub fn velocity_correction_all_wings(
        &self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        let span_lines = &line_force_model.span_lines_global;
        let nr_span_lines = span_lines.len();

        // Same interpolation as in the isolated version: the end points of each wing use the
        // velocity at the nearest control point, the inner points use the average.
        let span_point_velocities = line_force_model.span_point_values_from_ctrl_point_values(
            ctrl_points_velocity,
            false
        );

        let average_chord_length = line_force_model.chord_lengths.iter().sum::<Float>() /
            line_force_model.chord_lengths.len() as Float;

        let wake_length = average_chord_length * self.wake_length_factor;

        let wake_vectors: Vec<SpatialVector> = span_point_velocities.iter().map(|v| {
            wake_length * v.normalize()
        }).collect();

        let horseshoe_vortices_viscous = HorseshoeVortex::vortices_from_line_force_model_and_wake_vectors(
            line_force_model,
            &wake_vectors,
            self.viscous_core_length
        );

        let horseshoe_vortices_default = HorseshoeVortex::vortices_from_line_force_model_and_wake_vectors(
            line_force_model,
            &wake_vectors,
            self.viscous_core_length_reference
        );

        // The span points of each wing are stored after each other, with one more point than the
        // number of line elements in each wing
        let extension_vortices: Vec<(HorseshoeVortex, usize)> = (0..line_force_model.nr_wings())
            .flat_map(|wing_index| {
                let wing_indices = &line_force_model.wing_indices[wing_index];

                self.extension_vortices(
                    line_force_model,
                    wing_index,
                    [
                        span_point_velocities[wing_indices.start + wing_index],
                        span_point_velocities[wing_indices.end + wing_index]
                    ],
                    wake_length
                )
            })
            .collect();

        let mut u_i_correction: Vec<SpatialVector> = Vec::with_capacity(nr_span_lines);

        for span_line in span_lines.iter().take(nr_span_lines) {
            let ctrl_point = span_line.ctrl_point();
            let point_mirrored = self.symmetry_condition.mirrored_point(ctrl_point);

            let mut u_i_viscous = SpatialVector::default();
            let mut u_i_default = SpatialVector::default();

            for col_index in 0..nr_span_lines {
                let strength = circulation_strength[col_index];

                u_i_viscous += strength * self.unit_strength_induced_velocity(
                    &horseshoe_vortices_viscous[col_index],
                    ctrl_point,
                    point_mirrored
                );

                u_i_default += strength * self.unit_strength_induced_velocity(
                    &horseshoe_vortices_default[col_index],
                    ctrl_point,
                    point_mirrored
                );
            }

            u_i_default += self.extension_induced_velocity(
                &extension_vortices,
                circulation_strength,
                ctrl_point
            );

            u_i_correction.push(u_i_default - u_i_viscous);
        }

        u_i_correction
    }

    /// Creates the virtual extensions of a wing in the reference model, as one horseshoe vortex at
    /// each end with a length given by the virtual extension factors. Each vortex is returned
    /// together with the global index of the outermost line element, which it takes its circulation
    /// from. The inner trailing vortex of an extension then cancels the tip vortex of the wing, so
    /// that the tip vortex is moved to the end of the extension.
    ///
    /// The velocities are the velocities at the first and the last span point of the wing, which
    /// set the direction of the wake.
    fn extension_vortices(
        &self,
        line_force_model: &LineForceModel,
        wing_index: usize,
        end_span_point_velocities: [SpatialVector; 2],
        wake_length: Float,
    ) -> Vec<(HorseshoeVortex, usize)> {
        let wing_indices = &line_force_model.wing_indices[wing_index];
        let span_lines = &line_force_model.span_lines_global;

        let mut vortices = Vec::with_capacity(2);

        for end_index in 0..2 {
            let extension_factor = self.virtual_extension_factors[end_index];

            if extension_factor <= 0.0 {
                continue;
            }

            let line_index = if end_index == 0 {wing_indices.start} else {wing_indices.end - 1};

            let chord_length = line_force_model.chord_lengths[line_index];
            let extension_length = extension_factor * chord_length;

            // The extension continues the outermost line element, in the same direction
            let span_line = &span_lines[line_index];
            let direction = span_line.direction();

            let extension = if end_index == 0 {
                SpanLine {
                    start_point: span_line.start_point - extension_length * direction,
                    end_point: span_line.start_point,
                }
            } else {
                SpanLine {
                    start_point: span_line.end_point,
                    end_point: span_line.end_point + extension_length * direction,
                }
            };

            let wake_vector = wake_length * end_span_point_velocities[end_index].normalize();

            vortices.push((
                HorseshoeVortex::new(
                    &extension,
                    &[wake_vector, wake_vector],
                    chord_length * ELBOW_VORTEX_LENGTH_FACTOR,
                    self.viscous_core_length_reference
                ),
                line_index
            ));
        }

        vortices
    }

    /// The velocity induced by the virtual extensions at a point, including the effect of the
    /// symmetry condition
    fn extension_induced_velocity(
        &self,
        extension_vortices: &[(HorseshoeVortex, usize)],
        circulation_strength: &[Float],
        point: SpatialVector,
    ) -> SpatialVector {
        let point_mirrored = self.symmetry_condition.mirrored_point(point);

        extension_vortices.iter().map(|(vortex, line_index)| {
            circulation_strength[*line_index] * self.unit_strength_induced_velocity(
                vortex,
                point,
                point_mirrored
            )
        }).sum()
    }

    /// Induced velocity from a horseshoe vortex with unit strength, including the effect of the
    /// symmetry condition.
    fn unit_strength_induced_velocity(
        &self,
        vortex: &HorseshoeVortex,
        ctrl_point: SpatialVector,
        point_mirrored: Option<SpatialVector>,
    ) -> SpatialVector {
        let u_i = vortex.induced_velocity_with_unit_strength(ctrl_point);

        if let Some(p_m) = point_mirrored {
            let u_i_mirrored = vortex.induced_velocity_with_unit_strength(p_m);
            self.symmetry_condition.corrected_velocity(u_i, u_i_mirrored)
        } else {
            u_i
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::line_force_model::builder::LineForceModelBuilder;
    use crate::line_force_model::builder::single_wing::WingBuilder;
    use crate::section_models::SectionModel;
    use crate::section_models::foil::Foil;

    const CHORD_LENGTH: Float = 5.0;

    /// A single straight wing along the z-axis, with uniform circulation and a uniform velocity
    /// along the x-axis
    fn single_wing() -> (LineForceModel, Vec<SpatialVector>, Vec<Float>) {
        let mut builder = LineForceModelBuilder::new(16);

        builder.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::from([0.0, 0.0, 2.0]),
                SpatialVector::from([0.0, 0.0, 32.0]),
            ],
            chord_vectors: vec![SpatialVector::from([CHORD_LENGTH, 0.0, 0.0]); 2],
            section_model: SectionModel::SingleElementFoil(Foil::default()),
            ..Default::default()
        });

        let line_force_model = builder.build();
        let nr_span_lines = line_force_model.nr_span_lines();

        let velocity = vec![SpatialVector::from([8.0, 0.0, 0.0]); nr_span_lines];
        let circulation_strength = vec![10.0; nr_span_lines];

        (line_force_model, velocity, circulation_strength)
    }

    fn correction(
        builder: LiftingLineCorrectionBuilder,
        line_force_model: &LineForceModel,
        velocity: &[SpatialVector],
        circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        builder.build(0.6, line_force_model).velocity_correction(
            line_force_model, velocity, circulation_strength, 0.0
        )
    }

    #[test]
    fn max_correction_ratio_limits_the_correction() {
        let (line_force_model, velocity, circulation_strength) = single_wing();

        let max_correction_ratio = 0.01;

        let limited = correction(
            LiftingLineCorrectionBuilder {
                max_correction_ratio: Some(max_correction_ratio),
                ..Default::default()
            },
            &line_force_model, &velocity, &circulation_strength
        );

        let unlimited = correction(
            LiftingLineCorrectionBuilder::default(),
            &line_force_model, &velocity, &circulation_strength
        );

        for i in 0..limited.len() {
            let max_magnitude = max_correction_ratio * velocity[i].length();

            assert!(limited[i].length() <= max_magnitude * (1.0 + 1e-9));

            // The direction is kept
            assert!(limited[i].normalize().dot(unlimited[i].normalize()) > 1.0 - 1e-9);
        }

        // The limit is active at the ends, where the correction is largest
        assert!(unlimited[0].length() > max_correction_ratio * velocity[0].length());
    }

    #[test]
    fn virtual_extensions_move_the_tip_vortices() {
        let (line_force_model, velocity, circulation_strength) = single_wing();

        let without_extensions = correction(
            LiftingLineCorrectionBuilder::default(),
            &line_force_model, &velocity, &circulation_strength
        );

        let with_extensions = correction(
            LiftingLineCorrectionBuilder {
                virtual_extension_factors: [20.0, 20.0],
                ..Default::default()
            },
            &line_force_model, &velocity, &circulation_strength
        );

        let last = without_extensions.len() - 1;

        // The reference model is close to singular at the outermost control points without the
        // extensions, as the tip vortex is only half a line element away
        for index in [0, last] {
            assert!(with_extensions[index].length() < 0.2 * without_extensions[index].length());
        }
    }

    #[test]
    fn isolated_and_all_wings_corrections_are_equal_for_a_single_wing() {
        let (line_force_model, velocity, circulation_strength) = single_wing();

        let settings = |include_wing_interactions| LiftingLineCorrectionBuilder {
            virtual_extension_factors: [0.4, 1.0],
            symmetry_condition: SymmetryCondition::Z,
            include_wing_interactions,
            ..Default::default()
        };

        let isolated = correction(settings(false), &line_force_model, &velocity, &circulation_strength);
        let all_wings = correction(settings(true), &line_force_model, &velocity, &circulation_strength);

        for (u_isolated, u_all_wings) in isolated.iter().zip(&all_wings) {
            assert!((*u_isolated - *u_all_wings).length() < 1e-9 * (1.0 + u_isolated.length()));
        }
    }
}
