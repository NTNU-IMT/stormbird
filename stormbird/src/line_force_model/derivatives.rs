// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use self::motion::derivatives::Derivatives;

/// This implementation block contains functionality relevant for calculating velocities and 
/// accelerations due to motion and changing inflow conditions. This is necessary to generate input
/// to force calculations in the line force model.
impl LineForceModel {
    pub fn initialize_derivatives(&mut self, ctrl_points_freestream: &[SpatialVector<3>]) {
        let initial_angles = self.angles_of_attack(ctrl_points_freestream, CoordinateSystem::Global);

        self.derivatives = Some(
            Derivatives::new(
                self,
                ctrl_points_freestream,
                &initial_angles
            )
        )
    }

    pub fn felt_ctrl_points_freestream(&self, ctrl_points_freestream: &[SpatialVector<3>], time_step: f64) -> Vec<SpatialVector<3>> {
        let mut ctrl_point_velocity = Vec::with_capacity(ctrl_points_freestream.len());

        let motion_velocity = if let Some(derivatives) = &self.derivatives {
            derivatives.motion.ctrl_point_velocity(self, time_step)
        } else {
            vec![SpatialVector::<3>::default(); self.nr_span_lines()]
        };

        for i in 0..ctrl_points_freestream.len() {
            ctrl_point_velocity.push(
                ctrl_points_freestream[i] - motion_velocity[i]
            );
        }

        ctrl_point_velocity
    }

    pub fn update_derivatives(&mut self, current_velocity: &[SpatialVector<3>], current_angles_of_attack: &[f64]) {
        let ctrl_points = self.ctrl_points();
        let rotation = self.rigid_body_motion.rotation.clone();
        
        if let Some(derivatives) = self.derivatives.as_mut() {
            derivatives.motion.update(&ctrl_points, rotation);

            derivatives.flow.update(current_velocity, current_angles_of_attack);
        }
    }
}