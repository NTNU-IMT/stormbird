// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use std::ops::Range;

use stormath::type_aliases::Float;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
/// Numbers that describe the structure of the wake, used to convert between flat indices and
/// indices along the span and the stream
pub struct WakeIndices {
    /// Number of points along the span
    pub nr_points_along_span: usize,
    /// Number of panels along the span
    pub nr_panels_along_span: usize,
    /// Number of panels in the streamwise direction behind each line element
    pub nr_panels_per_line_element: usize,
    /// Number of points in the streamwise direction behind each line element
    pub nr_points_per_line_element: usize,
}

impl WakeIndices {
    #[inline(always)]
    /// Returns a flattened index for the wake panels. The panels are ordered streamwise-major.
    pub fn panel_index(&self, stream_index: usize, span_index: usize) -> usize {
        stream_index * self.nr_panels_along_span + span_index
    }

    #[inline(always)]
    /// Returns the stream and span indices from a flattened index
    pub fn reverse_panel_index(&self, flat_index: usize) -> (usize, usize) {
        let stream_index = flat_index / self.nr_panels_along_span;
        let span_index   = flat_index % self.nr_panels_along_span;

        (stream_index, span_index)
    }

    #[inline(always)]
    /// Returns a flattened index for the wake points. The points are ordered streamwise-major.
    pub fn point_index(&self, stream_index: usize, span_index: usize) -> usize {
        stream_index * self.nr_points_along_span + span_index
    }

    #[inline(always)]
    /// Return the total number of panels
    pub fn nr_panels(&self) -> usize {
        self.nr_panels_along_span * self.nr_panels_per_line_element
    }

    #[inline(always)]
    /// Return the total number of points
    pub fn nr_points(&self) -> usize {
        self.nr_points_along_span * self.nr_points_per_line_element
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
/// Enum to specify what type of direction to use when updating the first wake points, right behind 
/// the wing
pub enum FirstWakePointsDirection {
    /// The first panels behind the wing follows the chord direction. That is, the velocity 
    /// direction is not considered at all.
    Chord,
    #[default]
    /// The first panels follow the freestream, but without any induced velocities. 
    Freestream,
    /// The first panels follow the local velocity at each point, including induced velocities
    ActualVelocity
}

#[derive(Debug, Clone)]
/// Settings for the wake
pub struct WakeSettings {
    /// The length of the first panel, relative to the local chord length
    pub first_panel_relative_length: Float,
    /// The length of the last panel, relative to the local chord length
    pub last_panel_relative_length: Float,
    /// Which direction to use for the first wake points
    pub first_wake_points_direction: FirstWakePointsDirection,
    /// The flat indices of the wake points where the induced velocities are included in the stored
    /// velocity, see [`points_with_induced_velocities`]. Empty if the induced velocities are not
    /// used.
    pub points_with_induced_velocities: Range<usize>,
    /// The amount of damping in the shape of the wake
    pub shape_damping_factor: Float,
    /// Switch to set the strength update to be "steady-state"
    pub steady_state_strength_update: bool,
}
/// The flat indices of the wake points where the induced velocities must be included in the stored
/// velocity at the points. The induced velocities are only computed where the stored velocity is
/// used, which is in two places:
///
/// - Streaming of the free wake points: the velocity at row `k` moves the points to row `k + 1`.
///   The last row is set from the length of the last panel, so the velocities at the last two rows
///   are never used. The `ratio_of_wake_affected_by_induced_velocities` gives the fraction of the
///   rows `1..=N-2` that are affected, counted from the wing, where `N` is the number of panels per
///   line element.
/// - The [`ActualVelocity`](FirstWakePointsDirection::ActualVelocity) direction of the first free
///   row, which uses the average velocity at row 0 (the wing) and row 1. Both rows are then always
///   included.
///
/// The rows are stored after each other, so the result is one contiguous range of points.
pub fn points_with_induced_velocities(
    indices: &WakeIndices,
    ratio_of_wake_affected_by_induced_velocities: Float,
    first_wake_points_direction: FirstWakePointsDirection,
) -> Range<usize> {
    let nr_streamed_rows = indices.nr_panels_per_line_element.saturating_sub(2);

    let nr_affected_rows = (
        ratio_of_wake_affected_by_induced_velocities.clamp(0.0, 1.0) * nr_streamed_rows as Float
    ).ceil() as usize;

    let (start_row, end_row) = match first_wake_points_direction {
        FirstWakePointsDirection::ActualVelocity => (0, (1 + nr_affected_rows).max(2)),
        _ if nr_affected_rows == 0 => (0, 0),
        _ => (1, 1 + nr_affected_rows),
    };

    start_row * indices.nr_points_along_span..end_row * indices.nr_points_along_span
}
