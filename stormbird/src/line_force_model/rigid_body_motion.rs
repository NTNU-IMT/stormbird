// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use math_utils::spatial_vector::{
    SpatialVector,
    transformations::RotationType
};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Struct to represent fields necessary to define the motion of a rigid body.
/// 
/// Reference: https://en.wikipedia.org/wiki/Rigid_body_dynamics
pub struct RigidBodyMotion {
    pub translation: SpatialVector<3>,
    pub rotation: SpatialVector<3>,
    pub velocity_linear: SpatialVector<3>,
    pub velocity_angular: SpatialVector<3>,
    pub rotation_type: RotationType,
}

impl RigidBodyMotion {
    /// Applies the rigid body motion to a point in space
    pub fn transform_point(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        point.rotate(self.rotation, self.rotation_type) + self.translation
    }

    /// Applies the rigid body motion to a vector in space. That is, it applies the rotation.
    pub fn transform_vector(&self, vector: SpatialVector<3>) -> SpatialVector<3> {
        vector.rotate(self.rotation, self.rotation_type)
    }

    /// Returns the input vector in the body fixed coordinate system defined by the rigid body 
    /// motion.
    pub fn vector_in_body_fixed_coordinate_system(&self, vector: SpatialVector<3>) -> SpatialVector<3> {
        vector.in_rotated_coordinate_system(self.rotation, self.rotation_type)
    }
    
    /// Returns the relative position of the point to the center of the body.
    pub fn point_relative_to_body_center(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        point - self.translation
    }

    /// Computes the velocity at a point due to the motion of the rigid body. 
    pub fn velocity_at_point(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        self.velocity_linear + self.velocity_angular.cross(self.point_relative_to_body_center(point))
    }


    /// Function that sets new values for the translation and rotation, and then calculates the 
    /// derivatives (velocity and acceleration) using finite difference.
    pub fn update_translation_and_rotation_with_derivatives_using_finite_difference(
        &mut self, 
        translation: SpatialVector<3>, 
        rotation: SpatialVector<3>,
        time_step: f64
    ) {
        let old_translation = self.translation.clone();

        let old_rotation = self.rotation.clone();

        self.translation = translation;
        self.rotation = rotation;

        self.velocity_linear = (self.translation - old_translation) / time_step;
       
        self.velocity_angular = (self.rotation - old_rotation) / time_step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::f64::consts::PI;
    
    #[test]
    /// Test to compare the motion calculated by the rigid body motion struct against the a simpler, 
    /// but less accurate, finite difference method. The purpose is to validate the correctness of 
    /// the rigid body motion struct. It is assumed that any logic mistakes can be detected if the 
    /// difference against the finite difference method is large.
    fn compare_motion_against_finite_difference() {
        let rotation_amplitude = 45.0_f64.to_radians();
        let translation_amplitude = 2.1;

        let period = 0.5;

        let dt = period / 10_000.0;

        let angular_frequency = 2.0 * PI / period;

        let end_time = 0.5 * period;

        let rotation_motion = |t: f64| rotation_amplitude * (angular_frequency * t).sin();
        let rotation_motion_derivative = |t: f64| rotation_amplitude * angular_frequency * (angular_frequency * t).cos();
        
        let translation_motion = |t: f64| translation_amplitude * (angular_frequency * t).sin();
        let translation_motion_derivative = |t: f64| translation_amplitude * angular_frequency * (angular_frequency * t).cos();
    
        let initial_point_to_check = SpatialVector::<3>::new(0.0, 1.3, 0.8);

        let max_rotational_velocity = rotation_amplitude * angular_frequency;

        let mut transformed_points: Vec<SpatialVector<3>> = Vec::new();
        let mut motions: Vec<RigidBodyMotion> = Vec::new();
        
        let mut t = 0.0;
        while t < end_time {
            let current_motion = RigidBodyMotion {
                translation: SpatialVector([0.0, translation_motion(t), 0.0]),
                rotation: SpatialVector([rotation_motion(t), 0.0, 0.0]),
                velocity_linear: SpatialVector([0.0, translation_motion_derivative(t), 0.0]),
                velocity_angular: SpatialVector([rotation_motion_derivative(t), 0.0, 0.0]),
                rotation_type: RotationType::XYZ,
            };

            transformed_points.push(current_motion.transform_point(initial_point_to_check));
            motions.push(current_motion);

            t += dt;
        }

        for i in 1..transformed_points.len()-1 {
            let fd_velocity = (
                transformed_points[i + 1] - transformed_points[i - 1]
            ) / (2.0 * dt);

            let expected_velocity = motions[i].velocity_at_point(transformed_points[i]);

            let velocity_difference = (fd_velocity - expected_velocity).length();

            if i % 1000 == 0  {
                dbg!(fd_velocity, expected_velocity);
            }

            assert!(velocity_difference / max_rotational_velocity < 0.0001,
                "fd velocity: {}, rb velocity {}", fd_velocity, expected_velocity
            );

        }
     }
}