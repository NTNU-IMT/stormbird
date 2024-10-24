use crate::io_structs::prelude::CoordinateSystem;
use crate::line_force_model::span_line::SpanLine;

use super::*;

#[derive(Debug, Clone, Default)]
/// Structure for storing relevant data from the line force model that is needed for several methods
/// in the wake model.
pub struct LineForceModelData {
    pub span_lines: Vec<SpanLine>,
    pub chord_vectors: Vec<SpatialVector<3>>,
    pub felt_ctrl_points_freestream: Vec<SpatialVector<3>>,
    pub felt_ctrl_points_velocity: Vec<SpatialVector<3>>,
    pub angles_of_attack: Vec<f64>,
    pub amount_of_flow_separation: Vec<f64>,
    pub wake_angles: Vec<f64>,
    pub wing_indices: Vec<Range<usize>>,
}

impl LineForceModelData {
    pub fn new(
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector<3>],
        felt_ctrl_points_velocity: &[SpatialVector<3>]
    ) -> Self {
        let span_lines = line_force_model.span_lines();
        let chord_vectors = line_force_model.global_chord_vectors();

        let angles_of_attack = line_force_model.angles_of_attack(
            &felt_ctrl_points_velocity,
            CoordinateSystem::Global
        );

        let amount_of_flow_separation = line_force_model.amount_of_flow_separation(
            &angles_of_attack
        );

        let wake_angles = line_force_model.wake_angles(
            &felt_ctrl_points_velocity,
        );

        Self {
            span_lines,
            chord_vectors,
            felt_ctrl_points_freestream: felt_ctrl_points_freestream.to_vec(),
            felt_ctrl_points_velocity: felt_ctrl_points_velocity.to_vec(),
            angles_of_attack,
            amount_of_flow_separation,
            wake_angles,
            wing_indices: line_force_model.wing_indices.clone(),
        }
    }

    pub fn nr_span_lines(&self) -> usize {
        self.chord_vectors.len()
    }

    pub fn nr_wings(&self) -> usize {
        self.wing_indices.len()
    }

    /// Maps the values at the control points to the values at the span points using linear
    /// interpolation.
    pub fn span_point_values_from_ctrl_point_values<T>(
        &self,
        ctrl_point_values: &[T],
        extrapolate_ends: bool,
    ) -> Vec<T>
    where
        T: std::ops::Add<T, Output = T>
            + std::ops::Sub<T, Output = T>
            + std::ops::Mul<f64, Output = T>
            + Copy,
    {
        let mut span_point_values: Vec<T> =
            Vec::with_capacity(self.nr_span_lines() + self.nr_wings());

        for wing_index in 0..self.wing_indices.len() {
            let first_index = self.wing_indices[wing_index].start;

            // First point is extrapolated
            if extrapolate_ends {
                let first_delta =
                    ctrl_point_values[first_index] - ctrl_point_values[first_index + 1];

                span_point_values.push(ctrl_point_values[first_index] + first_delta);
            } else {
                span_point_values.push(ctrl_point_values[first_index]);
            }

            // Loop over all span lines in the wing
            for i in self.wing_indices[wing_index].clone() {
                let last_index = self.wing_indices[wing_index].clone().last().unwrap();

                // Last point is extrapolated, all others are interpolated
                if i == last_index {
                    if extrapolate_ends {
                        let last_delta =
                            ctrl_point_values[last_index] - ctrl_point_values[last_index - 1];
                        span_point_values.push(ctrl_point_values[last_index] + last_delta);
                    } else {
                        span_point_values.push(ctrl_point_values[last_index]);
                    }
                } else {
                    span_point_values.push((ctrl_point_values[i] + ctrl_point_values[i + 1]) * 0.5);
                }
            }
        }

        span_point_values
    }
}
