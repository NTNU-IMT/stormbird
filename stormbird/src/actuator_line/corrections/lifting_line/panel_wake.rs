// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality shared by the wake models of the lifting line correction that consist of vortex
//! panels.

use std::ops::Range;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::lifting_line::wake::dynamic_wake::DynamicWake;
use crate::lifting_line::wake::dynamic_wake::builder::{
    DynamicWakeBuilder,
    ViscousCoreLengthEvolution,
};
use crate::lifting_line::wake::settings::ViscousCoreLength;
use crate::lifting_line::wake::vtk::polygons_as_vtk;
use crate::line_force_model::LineForceModel;
use crate::error::Error;

use super::CorrectionSettings;

#[derive(Debug, Clone)]
/// Two panel wakes with the same geometry: one with a viscous core length equal to the force
/// projection width, and one with the small reference core length, which can increase downstream.
pub struct PanelWakePair {
    /// Wake with the viscous core length of the force projection width. Uses the circulation that
    /// the projected forces are computed from.
    pub viscous: DynamicWake,
    /// Wake with the small reference core length. Uses the circulation that the forces on the
    /// wings are computed from.
    pub reference: DynamicWake,
    /// Optional distance along the wake over which the reference core length grows to the viscous
    /// core length. Absolute value, i.e., not relative to the chord length.
    pub reference_core_evolution_length: Option<Float>,
}

impl PanelWakePair {
    /// Builds the two wakes from the settings in the `wake_builder`, where the viscous core length
    /// and the symmetry condition are set from the correction settings.
    pub fn new(
        wake_builder: DynamicWakeBuilder,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        reference_core_evolution_length_factor: Option<Float>,
    ) -> Result<Self, Error> {
        let viscous = DynamicWakeBuilder {
            viscous_core_length: ViscousCoreLength::Absolute(settings.viscous_core_length),
            symmetry_condition: settings.symmetry_condition,
            ..wake_builder.clone()
        }.build(line_force_model)?;

        let reference = DynamicWakeBuilder {
            viscous_core_length: ViscousCoreLength::Absolute(settings.reference_core_length),
            symmetry_condition: settings.symmetry_condition,
            ..wake_builder
        }.build(line_force_model)?;

        let reference_core_evolution_length = reference_core_evolution_length_factor.map(
            |factor| factor * line_force_model.average_chord_length()
        );

        Ok(Self {
            viscous,
            reference,
            reference_core_evolution_length,
        })
    }

    /// Updates the core lengths of the reference wake, if they evolve along the wake, and the panel
    /// data of both wakes from the current geometry. Must be called after the geometry is changed.
    pub fn update_panels(&mut self, settings: &CorrectionSettings) {
        if let Some(evolution_length) = self.reference_core_evolution_length {
            self.reference.panels_viscous_core_length = core_lengths_along_wake(
                &self.reference,
                settings.reference_core_length,
                settings.viscous_core_length,
                evolution_length
            );
        }

        self.viscous.update_panel_data();
        self.reference.update_panel_data();
    }

    /// The difference between the induced velocities from the reference wake and the viscous wake,
    /// at the control points of the line force model.
    ///
    /// The first `nr_rows_with_given_strength` streamwise rows of panels get the given circulation
    /// strength, while the rest of the panels use the strength stored in the wakes.
    pub fn induced_velocity_difference(
        &self,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
        nr_rows_with_given_strength: usize,
    ) -> Vec<SpatialVector> {
        let columns = columns_per_ctrl_point(settings, line_force_model);

        let u_i_reference = induced_velocities_at_ctrl_points(
            &self.reference,
            line_force_model,
            &columns,
            reference_circulation_strength,
            nr_rows_with_given_strength
        );

        let u_i_viscous = induced_velocities_at_ctrl_points(
            &self.viscous,
            line_force_model,
            &columns,
            projected_circulation_strength,
            nr_rows_with_given_strength
        );

        u_i_reference.iter().zip(&u_i_viscous)
            .map(|(&u_reference, &u_viscous)| u_reference - u_viscous)
            .collect()
    }
}

impl PanelWakePair {
    /// Serialize the geometry of the wakes as a legacy VTK PolyData buffer, with one quad per
    /// panel. The cell data is the strength and the core length of each panel in both wakes, where
    /// the first `nr_rows_with_given_strength` rows get the given circulation strength, as in
    /// [`induced_velocity_difference`](Self::induced_velocity_difference). The optional point
    /// velocities are written as point data, with the name `sampled_velocity`.
    pub fn as_vtk(
        &self,
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
        nr_rows_with_given_strength: usize,
        point_velocities: Option<&[SpatialVector]>,
        binary: bool,
    ) -> Vec<u8> {
        let indices = &self.reference.indices;

        let polygons: Vec<Vec<usize>> = (0..indices.nr_panels()).map(|panel_index| {
            let (stream_index, span_index) = indices.reverse_panel_index(panel_index);

            self.reference.panel_point_indices(stream_index, span_index).to_vec()
        }).collect();

        let strengths = |wake: &DynamicWake, given_strength: &[Float]| -> Vec<Float> {
            (0..indices.nr_panels()).map(|panel_index| {
                let (stream_index, span_index) = indices.reverse_panel_index(panel_index);

                if stream_index < nr_rows_with_given_strength {
                    given_strength[span_index]
                } else {
                    wake.strengths[panel_index]
                }
            }).collect()
        };

        let point_vectors: Vec<(&str, Vec<SpatialVector>)> = point_velocities
            .map(|velocities| vec![("sampled_velocity", velocities.to_vec())])
            .unwrap_or_default();

        polygons_as_vtk(
            "Stormbird lifting line correction wake",
            &self.reference.points,
            &polygons,
            &[
                ("reference_strength", strengths(&self.reference, reference_circulation_strength)),
                ("projected_strength", strengths(&self.viscous, projected_circulation_strength)),
                ("reference_core_length", self.reference.panels_viscous_core_length.clone()),
                ("viscous_core_length", self.viscous.panels_viscous_core_length.clone()),
            ],
            &point_vectors,
            binary
        )
    }
}

/// The range of line elements whose wakes are included at each control point: the line elements of
/// the same wing for isolated wings, which gives the self-induced velocities, or all line elements
/// with wing interactions.
fn columns_per_ctrl_point(
    settings: &CorrectionSettings,
    line_force_model: &LineForceModel,
) -> Vec<Range<usize>> {
    let nr_span_lines = line_force_model.nr_span_lines();

    line_force_model.wing_indices.iter().flat_map(|wing_range| {
        let columns = if settings.include_wing_interactions {
            0..nr_span_lines
        } else {
            wing_range.clone()
        };

        wing_range.clone().map(move |_| columns.clone())
    }).collect()
}

/// Induced velocities from a panel wake at the control points of the line force model, from the
/// panels shed from the line elements in the given column range for each control point.
///
/// The panels are visited in the order they are stored, one streamwise row at a time. The first
/// `nr_rows_with_given_strength` rows use the given circulation strength, and the rest use the
/// strength stored in the wake. The symmetry condition of the wake is included.
fn induced_velocities_at_ctrl_points(
    wake: &DynamicWake,
    line_force_model: &LineForceModel,
    columns: &[Range<usize>],
    circulation_strength: &[Float],
    nr_rows_with_given_strength: usize,
) -> Vec<SpatialVector> {
    let nr_cols = wake.indices.nr_panels_along_span;
    let nr_rows = wake.indices.nr_panels_per_line_element;

    let induced_velocity = |ctrl_point_index: usize| -> SpatialVector {
        let ctrl_point = line_force_model.span_lines_global[ctrl_point_index].ctrl_point();
        let columns = &columns[ctrl_point_index];

        let mut u_i = SpatialVector::default();

        for row_index in 0..nr_rows {
            let row_start = row_index * nr_cols;

            let strengths = if row_index < nr_rows_with_given_strength {
                circulation_strength
            } else {
                &wake.strengths[row_start..row_start + nr_cols]
            };

            for col_index in columns.clone() {
                let strength = strengths[col_index];

                if strength != 0.0 {
                    u_i += strength * wake.unit_strength_induced_velocity_from_panel_flat_index(
                        row_start + col_index,
                        ctrl_point
                    );
                }
            }
        }

        u_i
    };

    #[cfg(feature = "parallel")]
    let u_i: Vec<SpatialVector> = (0..columns.len()).into_par_iter().map(induced_velocity).collect();

    #[cfg(not(feature = "parallel"))]
    let u_i: Vec<SpatialVector> = (0..columns.len()).map(induced_velocity).collect();

    u_i
}

/// Viscous core length of each panel in the wake, increasing from the `start_value` to the
/// `end_value` along the wake, following
/// [`ViscousCoreLengthEvolution::sin_increase_factor`]. The position of a panel is the average
/// distance of its four corner points from the wing, measured along the streamwise lines of the
/// wake, so that the evolution is the same for straight and deformed wakes.
pub fn core_lengths_along_wake(
    wake: &DynamicWake,
    start_value: Float,
    end_value: Float,
    evolution_length: Float,
) -> Vec<Float> {
    let indices = &wake.indices;

    let mut distances = vec![0.0; indices.nr_points()];

    for i_stream in 1..indices.nr_points_per_line_element {
        for i_span in 0..indices.nr_points_along_span {
            let previous_index = indices.point_index(i_stream - 1, i_span);
            let index = indices.point_index(i_stream, i_span);

            distances[index] = distances[previous_index] +
                (wake.points[index] - wake.points[previous_index]).length();
        }
    }

    (0..indices.nr_panels()).map(|panel_index| {
        let (i_stream, i_span) = indices.reverse_panel_index(panel_index);

        let distance = wake.panel_point_indices(i_stream, i_span).iter()
            .map(|&point_index| distances[point_index])
            .sum::<Float>() / 4.0;

        let factor = ViscousCoreLengthEvolution::sin_increase_factor(distance / evolution_length);

        start_value * (1.0 - factor) + end_value * factor
    }).collect()
}
