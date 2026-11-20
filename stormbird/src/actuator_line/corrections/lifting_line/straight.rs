// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! The straight wake model of the lifting line correction, with one horseshoe vortex for each line
//! element.

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::lifting_line::singularity_elements::horseshoe_vortex::{
    HorseshoeVortex, ELBOW_VORTEX_LENGTH_FACTOR
};
use crate::line_force_model::LineForceModel;
use crate::line_force_model::span_line::SpanLine;

use super::CorrectionSettings;

#[derive(Debug, Clone)]
/// A straight wake, with one horseshoe vortex for each line element. The trailing vortices follow
/// the direction of the sampled velocity at each span point. The geometry is rebuilt in each call,
/// so the model has no state.
pub struct StraightWake {
    /// The length of the wake, relative to the average chord length
    pub wake_length_factor: Float,
    /// Length of the virtual extensions of each wing in the reference model, at the first and the
    /// last end, relative to the chord length at that end
    pub virtual_extension_factors: [Float; 2],
}

/// A virtual extension at the end of a wing
struct ExtensionVortex {
    /// The horseshoe vortex of the extension
    vortex: HorseshoeVortex,
    /// The global index of the outermost line element, which the extension takes its circulation
    /// from
    line_index: usize,
    /// The index of the wing that the extension belongs to
    wing_index: usize,
}

impl StraightWake {
    /// The difference between the induced velocities from the reference model and the model with
    /// the viscous core, at the control points of the line force model.
    ///
    /// For isolated wings, only the horseshoe vortices of the same wing as a control point are
    /// included, which gives the self-induced velocities. With wing interactions, all horseshoe
    /// vortices are included.
    pub fn induced_velocity_difference(
        &self,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        let span_lines = &line_force_model.span_lines_global;
        let nr_span_lines = span_lines.len();

        // The end points of each wing use the velocity at the nearest control point, the inner
        // points use the average.
        let span_point_velocities = line_force_model.span_point_values_from_ctrl_point_values(
            ctrl_points_velocity,
            false
        );

        let wake_length = line_force_model.average_chord_length() * self.wake_length_factor;

        let wake_vectors: Vec<SpatialVector> = span_point_velocities.iter().map(|v| {
            wake_length * v.normalize()
        }).collect();

        let vortices_viscous = HorseshoeVortex::vortices_from_line_force_model_and_wake_vectors(
            line_force_model,
            &wake_vectors,
            settings.viscous_core_length
        );

        let vortices_reference = HorseshoeVortex::vortices_from_line_force_model_and_wake_vectors(
            line_force_model,
            &wake_vectors,
            settings.reference_core_length
        );

        // The span points of each wing are stored after each other, with one more point than the
        // number of line elements in each wing
        let extension_vortices: Vec<ExtensionVortex> = (0..line_force_model.nr_wings())
            .flat_map(|wing_index| {
                let wing_indices = &line_force_model.wing_indices[wing_index];

                self.extension_vortices(
                    settings,
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

        let mut u_i_difference: Vec<SpatialVector> = Vec::with_capacity(nr_span_lines);

        for (wing_index, wing_range) in line_force_model.wing_indices.iter().enumerate() {
            let columns = if settings.include_wing_interactions {
                0..nr_span_lines
            } else {
                wing_range.clone()
            };

            for span_line in &span_lines[wing_range.clone()] {
                let ctrl_point = span_line.ctrl_point();

                let mut u_i_viscous = SpatialVector::default();
                let mut u_i_reference = SpatialVector::default();

                for col_index in columns.clone() {
                    u_i_viscous += projected_circulation_strength[col_index] *
                        settings.unit_strength_induced_velocity(&vortices_viscous[col_index], ctrl_point);

                    u_i_reference += reference_circulation_strength[col_index] *
                        settings.unit_strength_induced_velocity(&vortices_reference[col_index], ctrl_point);
                }

                for extension in &extension_vortices {
                    if settings.include_wing_interactions || extension.wing_index == wing_index {
                        u_i_reference += reference_circulation_strength[extension.line_index] *
                            settings.unit_strength_induced_velocity(&extension.vortex, ctrl_point);
                    }
                }

                u_i_difference.push(u_i_reference - u_i_viscous);
            }
        }

        u_i_difference
    }

    /// Creates the virtual extensions of a wing in the reference model, as one horseshoe vortex at
    /// each end with a length given by the virtual extension factors. The inner trailing vortex of
    /// an extension then cancels the tip vortex of the wing, so that the tip vortex is moved to the
    /// end of the extension.
    ///
    /// The velocities are the velocities at the first and the last span point of the wing, which
    /// set the direction of the wake.
    fn extension_vortices(
        &self,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        wing_index: usize,
        end_span_point_velocities: [SpatialVector; 2],
        wake_length: Float,
    ) -> Vec<ExtensionVortex> {
        let wing_indices = &line_force_model.wing_indices[wing_index];
        let span_lines = &line_force_model.span_lines_global;

        let mut vortices = Vec::with_capacity(2);

        for (end_index, end_span_point_velocity) in end_span_point_velocities.iter().enumerate() {
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

            let wake_vector = wake_length * end_span_point_velocity.normalize();

            vortices.push(ExtensionVortex {
                vortex: HorseshoeVortex::new(
                    &extension,
                    &[wake_vector, wake_vector],
                    chord_length * ELBOW_VORTEX_LENGTH_FACTOR,
                    settings.reference_core_length
                ),
                line_index,
                wing_index,
            });
        }

        vortices
    }
}
