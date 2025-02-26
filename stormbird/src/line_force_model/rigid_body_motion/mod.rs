// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use math_utils::spatial_vector::{
    SpatialVector,
    transformations::RotationType
};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    pub linear: SpatialVector<3>,
    pub angular: SpatialVector<3>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Struct to represent fields necessary to define the motion of a rigid body.
/// 
/// Reference: https://en.wikipedia.org/wiki/Rigid_body_dynamics
///
/// Can be used to calculate the transformation, velocity and acceleration of any point connected to a rigid body 
pub struct RigidBodyMotion {
    pub transformation: Motion,
    pub velocity: Motion,
    pub acceleration: Motion,
}

impl RigidBodyMotion {
    pub fn apply_transformation_to_point(&self, point: SpatialVector<3>, rotation_type: RotationType) -> SpatialVector<3> {
        point.rotate(self.transformation.angular, rotation_type) + self.transformation.linear
    }
    
    pub fn point_in_body_frame(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        // TODO: add rotation
        point - self.transformation.linear
    }

    pub fn velocity_at_point(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        self.velocity.linear + self.velocity.angular.cross(self.point_in_body_frame(point))
    }

    pub fn acceleration_at_point(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        let point_in_body_frame = self.point_in_body_frame(point);

        self.acceleration.linear + 
        self.acceleration.angular.cross(point_in_body_frame) + 
        self.velocity.angular.cross(self.velocity.angular.cross(point_in_body_frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::f64::consts::PI;
    
    #[test]
    fn rotational_velocity() {
        let amplitude = 45.0_f64.to_radians();
        let period = 0.5;

        let dt = period / 10_000.0;

        let angular_frequency = 2.0 * PI / period;

        let end_time = 0.5 * period;

        let motion = |t: f64| amplitude * (angular_frequency * t).sin();
        let motion_derivative = |t: f64| amplitude * angular_frequency * (angular_frequency * t).cos();
        let motion_second_derivative = |t: f64| -amplitude * angular_frequency.powi(2) * (angular_frequency * t).sin();
    
        let point_to_check = SpatialVector::<3>::new(0.0, 1.3, 0.8);

        let max_velocity = amplitude * angular_frequency;

        dbg!(max_velocity);

        let mut rotated_points: Vec<SpatialVector<3>> = Vec::new();
        let mut motions: Vec<RigidBodyMotion> = Vec::new();

        let rotation_type = RotationType::XYZ;
        
        let mut t = 0.0;
        while t < end_time {
            motions.push(
                RigidBodyMotion {
                    transformation: Motion {
                        linear: SpatialVector([0.0, 0.0, 0.0]),
                        angular: SpatialVector([motion(t), 0.0, 0.0]),
                    },
                    velocity: Motion {
                        linear: SpatialVector([0.0, 0.0, 0.0]),
                        angular: SpatialVector([motion_derivative(t), 0.0, 0.0]),
                    },
                    acceleration: Motion {
                        linear: SpatialVector([0.0, 0.0, 0.0]),
                        angular: SpatialVector([motion_second_derivative(t), 0.0, 0.0]),
                    },
                }
            );
            
            rotated_points.push(point_to_check.rotate(motions.last().unwrap().transformation.angular, rotation_type));

            t += dt;
        }

        for i in 1..rotated_points.len()-1 {
            let fd_velocity = (
                rotated_points[i + 1] - rotated_points[i - 1]
            ) / (2.0 * dt);

            let fd_acceleration = (
                rotated_points[i + 1] - 2.0 * rotated_points[i] + rotated_points[i - 1]
            ) / (dt * dt);

            let fd_angular_velocity = (
                motions[i + 1].transformation.angular - motions[i - 1].transformation.angular
            ) / (2.0 * dt);

            let expected_velocity = motions[i].velocity_at_point(rotated_points[i]);
            let expected_acceleration = motions[i].acceleration_at_point(rotated_points[i]);
            let expected_angular_velocity = motions[i].velocity.angular;

            let velocity_difference = (fd_velocity - expected_velocity).length();
            let acceleration_difference = (fd_acceleration - expected_acceleration).length();

            let angular_velocity_difference = (fd_angular_velocity - expected_angular_velocity).length();

            assert!(angular_velocity_difference < 0.001,
                "fd angular velocity: {}, rb angular velocity {}", fd_angular_velocity, expected_angular_velocity
            );

            assert!(velocity_difference / max_velocity < 0.0001,
                "fd velocity: {}, rb velocity {}", fd_velocity, expected_velocity
            );

            assert!(acceleration_difference / max_velocity < 0.0001,
                "fd acceleration: {}, rb acceleration {}", fd_acceleration, expected_acceleration
            );
        }
     }
}