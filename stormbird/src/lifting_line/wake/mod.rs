// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of wake models used to calculate induced velocities in lifting line simulations

pub mod settings;
pub mod builders;
pub mod export;
pub mod prelude;

pub mod frozen_wake;

pub mod induced_velocity_calc;
pub mod data_update;
pub mod line_force_model_data;

use line_force_model_data::LineForceModelData;

use stormath::spatial_vector::SpatialVector;

use crate::line_force_model::LineForceModel;

use rayon::prelude::*;
use rayon::iter::ParallelIterator;

use std::ops::Range;

use crate::lifting_line::singularity_elements::prelude::*;
use settings::*;

#[derive(Debug, Clone)]
/// Model of a wake for lifting line simulations
///
/// The induced velocities are calculated from vortex panels and their strengths.
///
/// The wake points and panels are assumed to be organized as a structured surface where indices
/// are stream wise-major. That means, the first panels right behind the wings are also the first
/// panels in the vector of panels in the wake.
///
/// A typical use case is as follows:
/// - For each time step, the points in the wake lying exactly on the wing lines are updated to
/// match the current wing geometry (which might have moved since the last time step)
/// - The strength of the first panel is then updated iteratively to solve the lifting line
/// equations. This happens in whatever solver that use this model. This model is used to calculate
/// the velocity as a function of the strength.
/// - When the strength for a time step is solved, the final velocity at the control points are
/// calculated.
/// - Finally, the wake points stream downstream, based on the current velocity field and time step.
///
/// There are methods to update the strength and the shape of the vortex line for each time step in
/// the simulation.
pub struct Wake {
    /// The indices for the wake
    pub indices: WakeIndices,
    /// The points making up the vortex wake
    pub points: Vec<SpatialVector<3>>,
    /// The strength of the panels without damping
    pub undamped_strengths: Vec<f64>,
    /// The strengths of the panels
    pub strengths: Vec<f64>,
    /// The life-time of the panels in the wake
    pub panels_lifetime: Vec<f64>,
    /// 'Viscosity' of each panel
    pub panels_strength_damping_factor: Vec<f64>,
    /// The viscous core length of each panel
    pub panels_viscous_core_length: Vec<f64>,
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
}

impl Wake {
    /// Returns the index of the wing that the span index belongs to
    fn wing_index(&self, span_index: usize) -> usize {
        for i in 0..self.wing_indices.len() {
            if self.wing_indices[i].contains(&span_index) {
                return i;
            }
        }

        panic!("Span index not found in any wing");
    }

    /// Returns the the indices to the four points that make up a panel at the given indices.
    ///
    /// The indices are ordered in a counter-clockwise manner. The first index is for the bottom
    /// left corner when viewing the panel from above.
    fn panel_point_indices(&self, panel_stream_index: usize, panel_span_index: usize) -> [usize; 4] {
        let wing_index = self.wing_index(panel_span_index);

        [
            self.indices.point_index(panel_stream_index, panel_span_index + wing_index),
            self.indices.point_index(panel_stream_index, panel_span_index + 1 + wing_index),
            self.indices.point_index(panel_stream_index + 1, panel_span_index + 1 + wing_index),
            self.indices.point_index(panel_stream_index + 1, panel_span_index + wing_index),
        ]
    }

    /// Returns the four points that make up a panel at the given indices
    fn panel_points(&self, panel_stream_index: usize, panel_span_index: usize) -> [SpatialVector<3>; 4] {
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
