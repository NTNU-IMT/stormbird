use crate::common_utils::prelude::CoordinateSystem;
use crate::line_force_model::{
    span_line::SpanLine,
    LineForceModel,
};

use super::*;

#[derive(Debug, Clone, Default)]
/// Structure for storing relevant data from the line force model that is needed for several methods
/// in the wake model. The reason for extracting this into a separate struct is that this data 
/// stores the values with all transformations applied. This avoids repeated calculations of the
/// same values in the wake model.
pub struct LineForceModelData {
    /// The span lines of the line force model, with both translation and rotation applied.
    pub span_lines: Vec<SpanLine>,
    /// The chord vectors of the line force model, with both translation and rotation applied.
    pub chord_vectors: Vec<SpatialVector>,
    /// The felt freestream velocity at the control points of the line force model. That is, 
    /// velocity due to the inflow free stream AND the motion of the line force model.
    pub felt_ctrl_points_freestream: Vec<SpatialVector>,
    /// The felt velocity at the control points of the line force model. That is, velocity due to 
    /// the inflow free stream AND the motion of the line force model AND the induced velocities
    pub felt_ctrl_points_velocity: Vec<SpatialVector>,
    /// The effective angles of attack at the control points of the line force model. 
    pub angles_of_attack: Vec<f64>,
    /// The amount of flow separation at the control points of the line force model. 
    pub amount_of_flow_separation: Vec<f64>,
    /// The angles of the wake at the control points of the line force model.
    pub wake_angles: Vec<f64>,
    pub wing_indices: Vec<Range<usize>>,
}

impl LineForceModelData {
    /// This function executes the right methods from the line force model to get the data needed to
    /// create the LineForceModelData, together with the supplied input data.
    /// 
    /// # Arguments
    /// * `line_force_model`: The line force model to get the data from.
    /// * `felt_ctrl_points_freestream`: The felt freestream velocity at the control points of 
    /// the line force model.
    /// * `felt_ctrl_points_velocity`: The felt velocity at the control points of the line force 
    /// model, including the induced velocities.
    pub fn new(
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector],
        felt_ctrl_points_velocity: &[SpatialVector]
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
        self.span_lines.len()
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
        LineForceModel::span_point_values_from_ctrl_point_values(
            &self.wing_indices,
            ctrl_point_values,
            extrapolate_ends
        )
    }
}
