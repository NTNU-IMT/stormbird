// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Export of the wake geometry
pub mod export;
/// Settings for the dynamic wake
pub mod settings;
/// Calculation of induced velocities from the wake
pub mod induced_velocity_calc;
/// Initialization of the wake
pub mod initialization;
/// Update of the wake geometry and strength in each time step
pub mod update_data;
pub mod builder;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use std::ops::Range;

use crate::lifting_line::singularity_elements::prelude::*;
use settings::*;

#[derive(Debug, Clone)]
/// Model of a dynamic wake for lifting line simulations.
///
/// The induced velocities are calculated from vortex panels and their strengths.
///
/// The wake points and panels are organized as a structured surface, stored as rows in the
/// streamwise direction. The stream index is the outer (slow) index and the span index is the inner
/// (fast) index, so the flat index is `stream_index * nr_along_span + span_index` (see
/// [`WakeIndices`]). Each row covers all wings, ordered as in the line force model. That means:
/// - The first row, i.e., the first `nr_panels_along_span` panels, are the panels right behind
///   the wings, which carry the circulation strength of the line elements.
/// - The panels of a single wing are not one contiguous range, but one contiguous segment in each
///   row.
/// - Each wing has one more point along the span than panels, so the span index of the points of
///   a panel is offset by the wing index.
///
/// A typical use case is as follows:
/// - For each time step, the points in the wake lying exactly on the wing lines are updated to
///   match the current wing geometry (which might have moved since the last time step), and the wake 
///   points stream downstream, based on the current velocity field and time step.
/// - The strength of the first panel is then updated iteratively to solve the lifting line
///   equations. This happens in whatever solver that use this model. This model is used to calculate
///   the velocity as a function of the strength.
/// - When the strength for a time step is solved, the final velocity at the control points are
///   calculated.
///
/// There are methods to update the strength and the shape of the vortex line for each time step in
/// the simulation.
pub struct DynamicWake {
    /// The indices for the wake
    pub indices: WakeIndices,
    /// The points making up the vortex wake
    pub points: Vec<SpatialVector>,
    /// The velocity at each point in the wake
    pub velocity_at_points: Vec<SpatialVector>,
    /// The strengths of the panels
    pub strengths: Vec<Float>,
    /// The viscous core length of each panel
    pub panels_viscous_core_length: Vec<Float>,
    /// Settings for the wake behavior
    pub settings: WakeSettings,
    /// The model used to calculate induced velocities from vortex lines
    pub potential_theory_settings: PotentialTheorySettings,
    /// To determine which wing the wake points belong to. Copied directly from the line force model
    pub wing_indices: Vec<Range<usize>>,
    /// Counter to keep track of the number of time steps that have been completed
    pub number_of_time_steps_completed: usize,
    /// Panel geometry data
    pub panels: Vec<Panel>,
    /// Representative chord length
    pub representative_chord_length: Float,
}

impl DynamicWake {
    /// Returns the index of the wing that the span index belongs to
    pub fn wing_index(&self, span_index: usize) -> usize {
        for i in 0..self.wing_indices.len() {
            if self.wing_indices[i].contains(&span_index) {
                return i;
            }
        }

        panic!("Span index not found in any wing");
    }

    /// Returns the indices to the four points that make up a panel at the given indices.
    ///
    /// The indices are ordered in a counter-clockwise manner. The first index is for the bottom
    /// left corner when viewing the panel from above.
    pub fn panel_point_indices(&self, panel_stream_index: usize, panel_span_index: usize) -> [usize; 4] {
        let wing_index = self.wing_index(panel_span_index);

        [
            self.indices.point_index(panel_stream_index, panel_span_index + wing_index),
            self.indices.point_index(panel_stream_index, panel_span_index + 1 + wing_index),
            self.indices.point_index(panel_stream_index + 1, panel_span_index + 1 + wing_index),
            self.indices.point_index(panel_stream_index + 1, panel_span_index + wing_index),
        ]
    }

    /// Returns the four points that make up a panel at the given indices
    fn panel_points(&self, panel_stream_index: usize, panel_span_index: usize) -> [SpatialVector; 4] {
        let point_indices = self.panel_point_indices(panel_stream_index, panel_span_index);

        [
            self.points[point_indices[0]],
            self.points[point_indices[1]],
            self.points[point_indices[2]],
            self.points[point_indices[3]],
        ]
    }
}

#[cfg(test)]
mod tests;
