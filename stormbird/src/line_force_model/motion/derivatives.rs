// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality to compute derivatives of different quantities relevant or the output from the 
//! line force model.

use math_utils::spatial_vector::SpatialVector;
use math_utils::finite_difference;
use crate::line_force_model::LineForceModel;

#[derive(Debug, Clone)]
pub struct Derivatives {
    pub motion: MotionDerivatives,
    pub flow: FlowDerivatives,
}

impl Derivatives {
    pub fn new(line_force_model: &LineForceModel, initial_velocity: &[SpatialVector<3>], initial_angles: &[f64]) -> Self {
        Self {
            motion: MotionDerivatives::new(line_force_model),
            flow: FlowDerivatives::new(initial_velocity, initial_angles),
        }
    }
}

#[derive(Debug, Clone)]
/// Functionality to calculate the motion of the control points of a line force model based on 
/// finite difference.
pub struct MotionDerivatives {
    /// Previous positions of the control points
    ctrl_points_history: [Vec<SpatialVector<3>>; 2],
    /// Previous values for the chord vector
    rotation_history: [SpatialVector<3>; 2],
    update_count: usize,
}

impl MotionDerivatives {
    pub fn new(line_force_model: &LineForceModel) -> Self {
        let ctrl_points = line_force_model.ctrl_points();
        
        Self {
            ctrl_points_history: [ctrl_points.clone(), ctrl_points],
            rotation_history: [line_force_model.rotation, line_force_model.rotation],
            update_count: 0,
        }
    }

    pub fn ctrl_point_velocity(&self, line_force_model: &LineForceModel, time_step: f64) -> Vec<SpatialVector<3>> {
        if line_force_model.nr_span_lines() != self.ctrl_points_history[0].len() {
            panic!(
                "The number of span lines in the line force model does not match the number of span lines in the force input calculator"
            );
        }

        if self.update_count < 2 {
            return vec![SpatialVector::<3>::default(); line_force_model.nr_span_lines()];
        }

        let current_ctrl_points = line_force_model.ctrl_points();

        let mut ctrl_point_velocity: Vec<SpatialVector<3>> = Vec::with_capacity(line_force_model.nr_span_lines());
        
        for i in 0..line_force_model.nr_span_lines() {
            let position_history = [
                self.ctrl_points_history[0][i],
                self.ctrl_points_history[1][i],
                current_ctrl_points[i]
            ];

            ctrl_point_velocity.push(
                finite_difference::first_derivative_second_order_backward(
                    &position_history, time_step
                )
            );
        }

        ctrl_point_velocity
    }

    pub fn rotation_velocity(&self, line_force_model: &LineForceModel, time_step: f64) -> SpatialVector<3> {
        let rotation_history = [
            self.rotation_history[0],
            self.rotation_history[1],
            line_force_model.rotation
        ];

        finite_difference::first_derivative_second_order_backward(
            &rotation_history, time_step
        )
    }

    pub fn update(&mut self, current_ctrl_points: &[SpatialVector<3>], current_rotation: SpatialVector<3>) {
        for i in 0..current_ctrl_points.len() {
            self.ctrl_points_history[0][i] = self.ctrl_points_history[1][i];
            self.ctrl_points_history[1][i] = current_ctrl_points[i];
        }

        self.rotation_history[0] = self.rotation_history[1];
        self.rotation_history[1] = current_rotation;

        self.update_count += 1;
    }
}

#[derive(Debug, Clone)]
/// Structure used to calculate the derivatives of flow quantities in a line force model
pub struct FlowDerivatives {
    pub velocity_history: [Vec<SpatialVector<3>>; 2],
    pub angles_of_attack_history: [Vec<f64>; 2],
    update_count: usize,
}

impl FlowDerivatives {
    /// Create a new FlowDerivativesCalculator
    pub fn new(initial_velocity: &[SpatialVector<3>], initial_angles: &[f64]) -> Self {
        if initial_velocity.len() != initial_angles.len() {
            panic!("The length of the initial velocity and initial angles must be the same");
        }

        Self {
            velocity_history: [initial_velocity.to_vec(), initial_velocity.to_vec()],
            angles_of_attack_history: [initial_angles.to_vec(), initial_angles.to_vec()],
            update_count: 0,
        }
    }

    pub fn acceleration(&self, current_velocity: &[SpatialVector<3>], time_step: f64) -> Vec<SpatialVector<3>> {
        if self.update_count < 2 {
            return vec![SpatialVector::<3>::default(); current_velocity.len()];
        }

        let mut acceleration = Vec::with_capacity(current_velocity.len());

        for i in 0..current_velocity.len() {
            let data = [
                self.velocity_history[0][i],
                self.velocity_history[1][i],
                current_velocity[i],
            ];

            acceleration.push(
                finite_difference::first_derivative_second_order_backward(
                    &data,
                    time_step,
                )
            );
        }

        acceleration
    }

    pub fn angles_of_attack_derivative(&self, current_angles_of_attack: &[f64], time_step: f64) -> Vec<f64> {
        if self.update_count < 2 {
            return vec![0.0; current_angles_of_attack.len()];
        }
        
        let mut angles_of_attack_derivative = Vec::with_capacity(current_angles_of_attack.len());

        for i in 0..current_angles_of_attack.len() {
            let data = [
                self.angles_of_attack_history[0][i],
                self.angles_of_attack_history[1][i],
                current_angles_of_attack[i],
            ];

            angles_of_attack_derivative.push(
                finite_difference::first_derivative_second_order_backward(
                    &data,
                    time_step,
                )
            );
        }

        angles_of_attack_derivative
    }

    pub fn update(&mut self, current_velocity: &[SpatialVector<3>], current_angles_of_attack: &[f64]) {
        self.velocity_history[0] = self.velocity_history[1].clone();
        self.velocity_history[1] = current_velocity.to_vec();

        self.angles_of_attack_history[0] = self.angles_of_attack_history[1].clone();
        self.angles_of_attack_history[1] = current_angles_of_attack.to_vec();

        self.update_count += 1;
    }
}