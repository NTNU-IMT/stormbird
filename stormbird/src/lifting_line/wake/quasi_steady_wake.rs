// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! The quasi-steady wake, consisting of one horseshoe vortex for each line element.

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::line_force_model::LineForceModel;
use crate::lifting_line::singularity_elements::horseshoe_vortex::HorseshoeVortex;

use super::settings::{QuasiSteadyWakeSettings, ViscousCoreLength};
use super::vtk::polygons_as_vtk;

#[derive(Clone, Debug)]
/// A quasi-steady wake, where each line element has a horseshoe vortex, with trailing vortices that
/// follow the velocity felt by the wings.
///
/// The geometry of the wake is rebuilt for every time step, based on the current state of the line
/// force model and the freestream velocity, while the strength of the horseshoe vortices is set to
/// the circulation strength from the solver after each time step. The wake therefore represents
/// the state at the last completed time step, which makes it possible to compute induced
/// velocities at arbitrary points and to export the wake geometry.
pub struct QuasiSteadyWake {
    /// The settings used to build the wake
    pub settings: QuasiSteadyWakeSettings,
    /// The horseshoe vortices, one for each line element. Empty before the first time step.
    pub horseshoe_vortices: Vec<HorseshoeVortex>,
    /// The strength of each horseshoe vortex, i.e., the circulation strength on each line element
    pub strengths: Vec<Float>,
}

impl QuasiSteadyWake {
    /// Creates an empty wake, which gets its geometry at the first time step
    pub fn new(settings: QuasiSteadyWakeSettings) -> Self {
        Self {
            settings,
            horseshoe_vortices: Vec::new(),
            strengths: Vec::new(),
        }
    }

    /// Rebuilds the horseshoe vortices from the current state of the line force model and the
    /// freestream velocity at the control points.
    ///
    /// The trailing vortices follow the velocity felt by the wings at the span points, i.e., the
    /// freestream velocity plus the velocity due to the motion of the wings. The length of the
    /// trailing vortices is the wake length factor times the average chord length. The strengths
    /// are set to zero if the number of line elements has changed.
    pub fn update_geometry(
        &mut self,
        line_force_model: &LineForceModel,
        ctrl_points_freestream: &[SpatialVector],
    ) {
        let nr_span_lines = line_force_model.span_lines_global.len();

        let span_lines = &line_force_model.span_lines_global;

        let average_chord_length = line_force_model.chord_lengths.iter().sum::<Float>() /
            (nr_span_lines as Float);

        let wake_length = self.settings.wake_length_factor * average_chord_length;

        // Add motion velocity
        let felt_ctrl_points_freestream = line_force_model.felt_ctrl_points_velocity(
            ctrl_points_freestream
        );

        let span_point_velocities = line_force_model.span_point_values_from_ctrl_point_values(
            &felt_ctrl_points_freestream,
            false
        );

        let wake_vectors: Vec<SpatialVector> = span_point_velocities.iter()
            .map(|velocity| velocity.normalize() * wake_length)
            .collect();

        let viscous_core_length = match self.settings.viscous_core_length {
            ViscousCoreLength::Relative(factor) => {
                let average_bound_vortex_length = span_lines.iter()
                    .map(|span_line| span_line.length())
                    .sum::<Float>() / (nr_span_lines as Float);

                factor * average_bound_vortex_length
            },
            ViscousCoreLength::Absolute(length) => length,
            ViscousCoreLength::NoViscousCore => 0.0,
        };

        self.horseshoe_vortices = HorseshoeVortex::vortices_from_line_force_model_and_wake_vectors(
            line_force_model,
            &wake_vectors,
            viscous_core_length
        );

        if self.strengths.len() != nr_span_lines {
            self.strengths = vec![0.0; nr_span_lines];
        }
    }

    /// Sets the strength of the horseshoe vortices, typically to the circulation strength from the
    /// solver after a time step.
    pub fn update_strengths(&mut self, strengths: &[Float]) {
        self.strengths = strengths.to_vec();
    }

    /// The induced velocity at a point from a single horseshoe vortex, assuming unit strength,
    /// including the effect of the symmetry condition.
    pub fn unit_strength_induced_velocity(
        &self,
        vortex_index: usize,
        point: SpatialVector
    ) -> SpatialVector {
        let vortex = &self.horseshoe_vortices[vortex_index];

        let u_i = vortex.induced_velocity_with_unit_strength(point);

        let symmetry_condition = self.settings.symmetry_condition;

        if let Some(point_mirrored) = symmetry_condition.mirrored_point(point) {
            let u_i_mirrored = vortex.induced_velocity_with_unit_strength(point_mirrored);

            symmetry_condition.corrected_velocity(u_i, u_i_mirrored)
        } else {
            u_i
        }
    }

    /// Calculates the induced velocities from all the horseshoe vortices at the given points, using
    /// the current strengths. Returns zero velocities before the first time step, as the wake is
    /// empty then.
    pub fn induced_velocities(&self, points: &[SpatialVector]) -> Vec<SpatialVector> {
        points.iter().map(|&point| {
            self.strengths.iter().enumerate()
                .filter(|(_, strength)| **strength != 0.0)
                .map(|(vortex_index, strength)| {
                    *strength * self.unit_strength_induced_velocity(vortex_index, point)
                })
                .fold(SpatialVector::default(), |sum, velocity| sum + velocity)
        }).collect()
    }

    /// Serialize the wake as a legacy VTK PolyData buffer, with one polygon per horseshoe vortex,
    /// and the strength and viscous core length of each horseshoe vortex as cell data.
    ///
    /// Each polygon goes along the vortex lines of the horseshoe vortex: from the far end of the
    /// first trailing vortex, along the bound vortex, to the far end of the second trailing vortex.
    /// The polygon is closed with a line between the two far ends, so that it represents the
    /// surface spanned by the horseshoe vortex. See [`polygons_as_vtk`] for more on the format.
    pub fn wake_as_vtk(&self, binary: bool) -> Vec<u8> {
        let nr_points_per_vortex = 6;

        let points: Vec<SpatialVector> = self.horseshoe_vortices.iter()
            .flat_map(|vortex| vortex.points())
            .collect();

        let polygons: Vec<Vec<usize>> = (0..self.horseshoe_vortices.len()).map(|vortex_index| {
            let start = vortex_index * nr_points_per_vortex;

            (start..start + nr_points_per_vortex).collect()
        }).collect();

        let viscous_core_lengths: Vec<Float> = self.horseshoe_vortices.iter()
            .map(|vortex| vortex.viscous_core_length())
            .collect();

        polygons_as_vtk(
            "Stormbird quasi-steady wake",
            &points,
            &polygons,
            &[
                ("strength", self.strengths.clone()),
                ("viscous_core_length", viscous_core_lengths),
            ],
            binary
        )
    }
}
