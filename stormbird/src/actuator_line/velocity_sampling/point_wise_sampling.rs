// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::vec3::Vec3;

#[derive(Debug, Clone)]
/// Structure to tor interpolated velocity at the control points in an actuator line model.
pub struct PointWiseVelocitySampling {
    pub ctrl_points_velocity: Vec<Vec3>,
}

impl PointWiseVelocitySampling{
    pub fn new(nr_line_elements: usize) -> Self {
        Self {
            ctrl_points_velocity: vec![Vec3::default(); nr_line_elements],
        }
    }

    pub fn reset(&mut self) {
        for i in 0..self.ctrl_points_velocity.len() {
            self.ctrl_points_velocity[i] = Vec3::default();
        }
    }
}