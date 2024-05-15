// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use crate::io_structs::derivatives::Derivatives;

/// This implementation block contains functionality relevant for calculating velocities and 
/// accelerations due to motion and changing inflow conditions. This is necessary to generate input
/// to force calculations in the line force model.
impl LineForceModel {
    pub fn initialize_derivatives(&mut self, ctrl_points_freestream: &[Vec3]) {
        let initial_angles = self.angles_of_attack(ctrl_points_freestream);

        self.derivatives = Some(
            Derivatives::new(
                self,
                ctrl_points_freestream,
                &initial_angles
            )
        )
    }

    pub fn felt_ctrl_points_freestream(&self, ctrl_points_freestream: &[Vec3], time_step: f64) -> Vec<Vec3> {
        let mut ctrl_point_velocity = Vec::with_capacity(ctrl_points_freestream.len());

        let motion_velocity = if let Some(derivatives) = &self.derivatives {
            derivatives.motion.ctrl_point_velocity(self, time_step)
        } else {
            vec![Vec3::default(); self.nr_span_lines()]
        };

        for i in 0..ctrl_points_freestream.len() {
            ctrl_point_velocity.push(
                ctrl_points_freestream[i] - motion_velocity[i]
            );
        }

        ctrl_point_velocity
    }

    pub fn update_derivatives(&mut self, current_velocity: &[Vec3], current_angles_of_attack: &[f64]) {
        let ctrl_points = self.ctrl_points();
        let rotation = self.rotation.clone();
        
        if let Some(derivatives) = self.derivatives.as_mut() {
            derivatives.motion.update(&ctrl_points, rotation);

            derivatives.flow.update(current_velocity, current_angles_of_attack);
        }
    }

    pub fn sectional_force_input(&self, solver_result: &SolverResult, time_step: f64) -> SectionalForcesInput {
        let angles_of_attack = self.angles_of_attack(&solver_result.ctrl_point_velocity);

        let mut acceleration = vec![Vec3::default(); self.nr_span_lines()];
        let mut angles_of_attack_derivative = vec![0.0; self.nr_span_lines()];
        let mut rotation_velocity = Vec3::default();

        if let Some(derivatives) = &self.derivatives {
            acceleration = derivatives.flow.acceleration(
                &solver_result.ctrl_point_velocity, time_step
            );

            angles_of_attack_derivative = derivatives.flow.angles_of_attack_derivative(
                &angles_of_attack, time_step
            );

            rotation_velocity = derivatives.motion.rotation_velocity(self, time_step);
        }

        SectionalForcesInput {
            circulation_strength: solver_result.circulation_strength.clone(),
            velocity: solver_result.ctrl_point_velocity.clone(),
            angles_of_attack,
            acceleration,
            angles_of_attack_derivative,
            rotation_velocity
        }
    }

}