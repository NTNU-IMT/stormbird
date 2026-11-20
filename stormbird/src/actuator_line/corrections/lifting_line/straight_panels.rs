// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! The straight panel wake model of the lifting line correction.

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::lifting_line::wake::dynamic_wake::builder::DynamicWakeBuilder;
use crate::line_force_model::LineForceModel;
use crate::error::Error;

use super::CorrectionSettings;
use super::builder::StraightPanelWakeBuilder;
use super::panel_wake::PanelWakePair;

#[derive(Debug, Clone)]
/// A straight wake, split into panels in the streamwise direction. All panels behind a line element
/// have the circulation of the line element, so the wake is equivalent to a horseshoe vortex, but
/// the reference core length can increase downstream.
///
/// The wake follows the direction of the sampled velocity at each span point. The distance from the
/// wing to the streamwise point `k` is `L (k / N)^p`, where `L` is the wake length and `N` the
/// number of panels. The exponent `p` is set so that the first panel has the given length, which
/// gives panels that get gradually longer downstream when the first panel is shorter than the
/// average panel length.
///
/// The geometry is rebuilt in each call to [`update_geometry`](Self::update_geometry), which must
/// be called before the induced velocities are computed.
pub struct StraightPanelWake {
    /// The length of the wake, relative to the average chord length
    pub wake_length_factor: Float,
    /// Length of the first panel, relative to the average chord length
    pub first_panel_relative_length: Float,
    /// The panel wakes with the viscous and the reference core length
    pub wakes: PanelWakePair,
    /// Whether the geometry has been set from the sampled velocities
    geometry_is_set: bool,
}

impl StraightPanelWake {
    /// Creates the wake from the builder
    pub fn new(
        builder: &StraightPanelWakeBuilder,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
    ) -> Result<Self, Error> {
        let wake_builder = DynamicWakeBuilder {
            nr_panels_per_line_element: builder.nr_panels_per_line_element,
            far_field_ratio: builder.far_field_ratio,
            steady_state_strength_update: true,
            ..Default::default()
        };

        let wakes = PanelWakePair::new(
            wake_builder,
            settings,
            line_force_model,
            builder.reference_core_evolution_length_factor
        )?;

        Ok(Self {
            wake_length_factor: builder.wake_length_factor,
            first_panel_relative_length: builder.first_panel_relative_length,
            wakes,
            geometry_is_set: false,
        })
    }

    /// The distance from the wing to each streamwise row of points, relative to the wake length
    fn relative_point_distances(&self, nr_panels: usize) -> Vec<Float> {
        let relative_first_panel_length = self.first_panel_relative_length / self.wake_length_factor;

        let exponent = relative_first_panel_length.ln() / (1.0 / nr_panels as Float).ln();

        (0..=nr_panels).map(|k| (k as Float / nr_panels as Float).powf(exponent)).collect()
    }

    /// Rebuilds the geometry of the wakes, so that they follow the direction of the sampled
    /// velocity at the span points.
    pub fn update_geometry(
        &mut self,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
    ) {
        let indices = self.wakes.reference.indices.clone();

        let wake_length = self.wake_length_factor * line_force_model.average_chord_length();

        let relative_distances = self.relative_point_distances(indices.nr_panels_per_line_element);

        // The end points of each wing use the velocity at the nearest control point, the inner
        // points use the average, as for the straight wake
        let directions: Vec<SpatialVector> = line_force_model.span_point_values_from_ctrl_point_values(
            ctrl_points_velocity,
            false
        ).iter().map(|velocity| velocity.normalize()).collect();

        let span_points = &line_force_model.span_points_global;

        for wake in [&mut self.wakes.viscous, &mut self.wakes.reference] {
            for (i_stream, relative_distance) in relative_distances.iter().enumerate() {
                for i_span in 0..indices.nr_points_along_span {
                    wake.points[indices.point_index(i_stream, i_span)] = span_points[i_span] +
                        directions[i_span] * (relative_distance * wake_length);
                }
            }
        }

        self.wakes.update_panels(settings);

        self.geometry_is_set = true;
    }

    /// The wake as a legacy VTK PolyData buffer, see [`PanelWakePair::as_vtk`]. All panels have the
    /// given circulation. None if the geometry has not been set yet.
    pub fn as_vtk(
        &self,
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
        binary: bool,
    ) -> Option<Vec<u8>> {
        if !self.geometry_is_set {
            return None;
        }

        Some(self.wakes.as_vtk(
            reference_circulation_strength,
            projected_circulation_strength,
            self.wakes.reference.indices.nr_panels_per_line_element,
            None,
            binary
        ))
    }

    /// The difference between the induced velocities from the reference wake and the viscous wake,
    /// at the control points of the line force model.
    pub fn induced_velocity_difference(
        &self,
        settings: &CorrectionSettings,
        line_force_model: &LineForceModel,
        reference_circulation_strength: &[Float],
        projected_circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        assert!(
            self.geometry_is_set,
            "The geometry of the straight panel wake must be updated before the correction is \
            computed"
        );

        // All rows of panels have the circulation of the line element they are shed from
        let nr_rows = self.wakes.reference.indices.nr_panels_per_line_element;

        self.wakes.induced_velocity_difference(
            settings,
            line_force_model,
            reference_circulation_strength,
            projected_circulation_strength,
            nr_rows
        )
    }
}
