// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::vec3::Vec3;
use crate::line_force_model::LineForceModel;

use crate::math_utils::finite_difference;

#[derive(Debug, Clone)]
/// Structure to store necessary data to calculate the forces due to motion of wings on a line 
/// force model. The data is assumed to represent the motion of the wing and not the *felt* velocity
/// at the wings. The conversion between wing motion and felt motion must be done elsewhere.
/// 
/// Each vector in this struct represents values at control points for the line force model.
pub struct Motion {
    /// Velocity due to wing motion at the control points
    pub velocity: Vec<Vec3>,
    /// Acceleration due to wing motion at the control points. Mainly used for calculating added mass forces
    pub acceleration: Vec<Vec3>,
    /// How fast the chord vector is rotating. Mainly used to calculate additional lift due to this
    /// rotation
    pub chord_rotation_velocity: Vec<f64>,
}

impl Motion {
    pub fn new_zero(nr_span_lines: usize) -> Self {
        Self {
            velocity: vec![Vec3::default(); nr_span_lines],
            acceleration: vec![Vec3::default(); nr_span_lines],
            chord_rotation_velocity: vec![0.0; nr_span_lines],
        }
    }
}

#[derive(Debug, Clone)]
/// Functionality to calculate the motion of the control points of a line force model based on 
/// finite difference.
pub struct MotionCalculator {
    /// Previous positions of the control points
    ctrl_points_history: [Vec<Vec3>; 2],
    /// Previous values for the chord vector
    chord_vector_history: [Vec<Vec3>; 2],
}

impl MotionCalculator {
    pub fn new(line_force_model: &LineForceModel) -> Self {
        let ctrl_points = line_force_model.ctrl_points();
        let chord_vectors = line_force_model.chord_vectors();
        
        Self {
            ctrl_points_history: [ctrl_points.clone(), ctrl_points.clone()],
            chord_vector_history: [chord_vectors.clone(), chord_vectors.clone()],
        }
    }

    /// Calculates the force input for a given time step.
    /// 
    /// The velocity and acceleration at each control point is estimated from finite difference 
    /// calculation of the stored values of ctrl points and chord vectors.
    pub fn get_motion(
        &mut self, 
        line_force_model: &LineForceModel, 
        time_step: f64
    ) -> Motion {
        if line_force_model.nr_span_lines() != self.ctrl_points_history[0].len() {
            panic!(
                "The number of span lines in the line force model does not match the number of span lines in the force input calculator"
            );
        }

        let current_ctrl_points = line_force_model.ctrl_points();
        let current_chord_vectors = line_force_model.chord_vectors();

        let mut velocity: Vec<Vec3> = Vec::with_capacity(line_force_model.nr_span_lines());
        let mut acceleration: Vec<Vec3> = Vec::with_capacity(line_force_model.nr_span_lines());
        
        for i in 0..line_force_model.nr_span_lines() {
            let position_history = [
                self.ctrl_points_history[0][i],
                self.ctrl_points_history[1][i],
                current_ctrl_points[i]
            ];

            velocity.push(
                finite_difference::first_derivative_second_order_backward(
                    &position_history, time_step
                )
            );

            acceleration.push(
                finite_difference::second_derivative_backward(
                    &position_history, time_step
                )
            );
        }

        let chord_rotation_velocity = vec![0.0; line_force_model.nr_span_lines()];

        for i in 0..line_force_model.nr_span_lines() {
            self.ctrl_points_history[0][i] = self.ctrl_points_history[1][i];
            self.ctrl_points_history[1][i] = current_ctrl_points[i];

            self.chord_vector_history[0][i] = self.chord_vector_history[1][i];
            self.chord_vector_history[1][i] = current_chord_vectors[i];
        }

        Motion {
            velocity,
            acceleration,
            chord_rotation_velocity,
        }
    }
}