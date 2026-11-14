// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! General functionality to represent rigid body motion. That is, a data structure and methods for
//! storing and relating translation and rotation.

use crate::spatial_vector::{
    SpatialVector,
    transformations::RotationType
};

use crate::type_aliases::{Float};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Struct to represent fields necessary to define the motion of a rigid body.
///
/// Reference: <https://en.wikipedia.org/wiki/Rigid_body_dynamics>
pub struct RigidBodyMotion {
    /// Translation of the body
    pub translation: SpatialVector,
    /// Rotation of the body, as angles around the x, y and z axes
    pub rotation: SpatialVector,
    /// Linear velocity of the body
    pub velocity_linear: SpatialVector,
    /// Angular velocity of the body
    pub velocity_angular: SpatialVector,
    /// How the rotation angles are applied
    pub rotation_type: RotationType,
}

impl RigidBodyMotion {
    /// Applies the rigid body motion to a point in space
    pub fn transform_point(&self, point: SpatialVector) -> SpatialVector {
        point.rotate(self.rotation, self.rotation_type) + self.translation
    }

    /// Applies the rigid body motion to a vector in space. That is, it applies the rotation.
    pub fn transform_vector(&self, vector: SpatialVector) -> SpatialVector {
        vector.rotate(self.rotation, self.rotation_type)
    }

    /// Returns the input vector in the body fixed coordinate system defined by the rigid body
    /// motion.
    pub fn vector_in_body_fixed_coordinate_system(&self, vector: SpatialVector) -> SpatialVector {
        vector.in_rotated_coordinate_system(self.rotation, self.rotation_type)
    }

    /// Returns the input vector, given in the body fixed coordinate system defined by the rigid
    /// body motion, in the global coordinate system. This is the inverse of
    /// [`vector_in_body_fixed_coordinate_system`](Self::vector_in_body_fixed_coordinate_system).
    /// It can, for instance, be used to convert an angular velocity in body fixed axes, often
    /// called (p, q, r) for ships, to the global angular velocity vector.
    pub fn vector_from_body_fixed_to_global_coordinate_system(
        &self,
        vector: SpatialVector
    ) -> SpatialVector {
        vector.from_rotated_to_global_system(self.rotation, self.rotation_type)
    }

    /// Returns the relative position of the point to the center of the body.
    pub fn point_relative_to_body_center(&self, point: SpatialVector) -> SpatialVector {
        point - self.translation
    }

    /// Return the velocity due to motion alone at the point
    pub fn rotation_velocity_at_point(&self, point: SpatialVector) -> SpatialVector {
        self.velocity_angular.cross(self.point_relative_to_body_center(point))
    }

    /// Computes the velocity at a point due to the motion of the rigid body.
    pub fn velocity_at_point(&self, point: SpatialVector) -> SpatialVector {
        self.velocity_linear + self.rotation_velocity_at_point(point)
    }

    /// Computes the velocity at several points due to the motion of the rigid body.
    pub fn velocities_at_points(&self, points: &[SpatialVector]) -> Vec<SpatialVector> {
        points.iter()
            .map(|point| self.velocity_at_point(*point))
            .collect()
    }

    /// Sets a new translation, and updates the linear velocity with a first order finite difference
    pub fn update_translation_with_velocity_using_finite_difference(
        &mut self,
        translation: SpatialVector,
        time_step: Float
    ) {
        let old_translation = self.translation;

        self.translation = translation;

        self.velocity_linear = (self.translation - old_translation) / time_step;
    }

    /// Sets a new rotation, and updates the angular velocity with a first order finite difference,
    /// see [`angular_velocity_from_rotation_change`].
    pub fn update_rotation_with_velocity_using_finite_difference(
        &mut self,
        rotation: SpatialVector,
        time_step: Float
    ) {
        let old_rotation = self.rotation;

        self.rotation = rotation;

        self.velocity_angular = angular_velocity_from_rotation_change(
            old_rotation,
            self.rotation,
            time_step,
            self.rotation_type
        );
    }
}

/// Returns the angular velocity vector, in the global coordinate system, of a body that rotates
/// from the old to the new rotation during the given time step.
///
/// The rotations are given as angles around the x, y and z axes, applied as defined by the rotation
/// type. Note that the angular velocity is *not* the time derivative of these angles, except when
/// the body only rotates around a single axis. For instance, for the `XYZ` rotation type, where
/// R = Rz Ry Rx, the angular velocity is
///
/// ω = φ̇ (Rz Ry ex) + θ̇ (Rz ey) + ψ̇ ez
///
/// where φ, θ and ψ are the angles around the x, y and z axes. That is, the rate of the first
/// rotation acts around an axis that is rotated by the following rotations.
///
/// The angular velocity is computed from the rotation matrices rather than from the angles. The
/// change in rotation during the time step, ΔR = R_new R_oldᵀ, is converted to a rotation angle
/// around a single axis, and the angular velocity is the axis times the angle divided by the time
/// step. This is exact for a constant angular velocity during the time step, works for all rotation
/// types, and handles angles that jump between -π and π. It assumes that the body rotates less than
/// π during one time step.
pub fn angular_velocity_from_rotation_change(
    old_rotation: SpatialVector,
    new_rotation: SpatialVector,
    time_step: Float,
    rotation_type: RotationType
) -> SpatialVector {
    let unit_vectors = [SpatialVector::unit_x(), SpatialVector::unit_y(), SpatialVector::unit_z()];

    // Columns of the old rotation matrix, i.e., the rotated unit vectors
    let old_columns = unit_vectors.map(|unit| unit.rotate(old_rotation, rotation_type));

    // ΔR = R_new R_oldᵀ. Column j of ΔR is R_new applied to row j of R_old, as R_oldᵀ e_j is row j
    // of R_old.
    let delta_columns: [SpatialVector; 3] = std::array::from_fn(|j| {
        let old_row = SpatialVector::new(old_columns[0][j], old_columns[1][j], old_columns[2][j]);

        old_row.rotate(new_rotation, rotation_type)
    });

    // Element (row, col) of ΔR
    let m = |row: usize, col: usize| delta_columns[col][row];

    let trace = m(0, 0) + m(1, 1) + m(2, 2);
    let cos_angle = (0.5 * (trace - 1.0)).clamp(-1.0, 1.0);
    let angle = cos_angle.acos();

    // The skew-symmetric part of ΔR is sin(angle) times the cross product matrix of the axis
    let sin_angle_times_axis = 0.5 * SpatialVector::new(
        m(2, 1) - m(1, 2),
        m(0, 2) - m(2, 0),
        m(1, 0) - m(0, 1),
    );

    let sin_angle = angle.sin();

    let rotation_vector = if angle < 1e-6 {
        // Small angles: sin(angle) ≈ angle
        sin_angle_times_axis
    } else if sin_angle > 1e-6 {
        sin_angle_times_axis * (angle / sin_angle)
    } else {
        // The angle is close to π, where the skew-symmetric part vanishes. The axis is then found
        // from the symmetric part, ΔR = 2 a aᵀ - I. The sign of the axis does not matter for a
        // rotation of π.
        let b = |i: usize| (0.5 * (m(i, i) + 1.0)).max(0.0);

        let largest = (0..3).max_by(|&i, &j| b(i).total_cmp(&b(j))).unwrap_or(0);
        let a_largest = b(largest).sqrt();

        let mut axis = SpatialVector::default();

        for i in 0..3 {
            axis[i] = if i == largest {
                a_largest
            } else {
                0.5 * (m(i, largest) + m(largest, i)) / (2.0 * a_largest)
            };
        }

        axis.normalize() * angle
    };

    rotation_vector / time_step
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::consts::PI;
    use crate::type_aliases::Float;

    #[test]
    /// Test to compare the motion calculated by the rigid body motion struct against the a simpler,
    /// but less accurate, finite difference method. The purpose is to validate the correctness of
    /// the rigid body motion struct. It is assumed that any logic mistakes can be detected if the
    /// difference against the finite difference method is large.
    fn compare_motion_against_finite_difference() {
        let rotation_amplitude = Float::to_radians(45.0);
        let translation_amplitude = 2.1;

        let period = 0.5;

        let dt = period / 10_000.0;

        let angular_frequency = 2.0 * PI / period;

        let end_time = 0.5 * period;

        let rotation_motion = |t: Float| rotation_amplitude * (angular_frequency * t).sin();
        let rotation_motion_derivative = |t: Float| rotation_amplitude * angular_frequency * (angular_frequency * t).cos();

        let translation_motion = |t: Float| translation_amplitude * (angular_frequency * t).sin();
        let translation_motion_derivative = |t: Float| translation_amplitude * angular_frequency * (angular_frequency * t).cos();

        let initial_point_to_check = SpatialVector::new(0.0, 1.3, 0.8);

        let max_rotational_velocity = rotation_amplitude * angular_frequency;

        let mut transformed_points: Vec<SpatialVector> = Vec::new();
        let mut motions: Vec<RigidBodyMotion> = Vec::new();

        let mut t = 0.0;
        while t < end_time {
            let current_motion = RigidBodyMotion {
                translation: SpatialVector::from([0.0, translation_motion(t), 0.0]),
                rotation: SpatialVector::from([rotation_motion(t), 0.0, 0.0]),
                velocity_linear: SpatialVector::from([0.0, translation_motion_derivative(t), 0.0]),
                velocity_angular: SpatialVector::from([rotation_motion_derivative(t), 0.0, 0.0]),
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

            assert!(velocity_difference / max_rotational_velocity < 0.0008,
                "fd velocity: {}, rb velocity {}", fd_velocity, expected_velocity
            );

        }
     }

    /// Returns the tolerance to use in a test, depending on whether the library is compiled with
    /// double or single precision
    fn tolerance(double_precision: Float, single_precision: Float) -> Float {
        if cfg!(feature = "single_precision") {
            single_precision
        } else {
            double_precision
        }
    }

    /// All rotation types, for tests that should hold for any of them
    const ROTATION_TYPES: [RotationType; 6] = [
        RotationType::XYZ, RotationType::XZY, RotationType::YXZ,
        RotationType::YZX, RotationType::ZXY, RotationType::ZYX,
    ];

    #[test]
    /// Example from the documentation: a ship with a heading of 90 degrees that rolls. The roll
    /// axis is the x-axis of the ship, which points along the global y-axis.
    fn roll_with_heading_rotates_around_the_ship_axis() {
        let heading = Float::to_radians(90.0);
        let roll_rate = 0.1;
        let time_step = 0.01;

        let mut motion = RigidBodyMotion {
            rotation: SpatialVector::new(0.0, 0.0, heading),
            rotation_type: RotationType::XYZ,
            ..Default::default()
        };

        motion.update_rotation_with_velocity_using_finite_difference(
            SpatialVector::new(roll_rate * time_step, 0.0, heading),
            time_step
        );

        let expected = SpatialVector::new(0.0, roll_rate, 0.0);

        assert!(
            (motion.velocity_angular - expected).length() < tolerance(1e-9, 1e-6),
            "angular velocity: {:?}", motion.velocity_angular
        );
    }

    #[test]
    /// Compares the velocity at a point, from the angular velocity, with a central finite
    /// difference of the position of the point, for a motion where all three angles vary, a large
    /// constant heading, and all rotation types
    fn velocity_at_point_with_combined_rotations() {
        // The finite difference of the positions is the reference. A larger step is used in single
        // precision, where round-off errors would otherwise dominate the reference.
        let time_step = tolerance(1e-4, 1e-2);
        let point_local = SpatialVector::new(3.0, -2.0, 30.0);

        let angles = |t: Float| SpatialVector::new(
            Float::to_radians(10.0) * (1.3 * t).sin(),
            Float::to_radians(5.0) * (0.7 * t).cos(),
            Float::to_radians(135.0) + Float::to_radians(8.0) * (0.4 * t).sin(),
        );

        for rotation_type in ROTATION_TYPES {
            for t in [0.0, 0.8, 2.1, 3.7] {
                let mut motion = RigidBodyMotion {
                    rotation: angles(t - time_step),
                    rotation_type,
                    ..Default::default()
                };

                motion.update_rotation_with_velocity_using_finite_difference(
                    angles(t + time_step),
                    2.0 * time_step
                );

                let position = |t: Float| point_local.rotate(angles(t), rotation_type);

                let fd_velocity = (position(t + time_step) - position(t - time_step)) /
                    (2.0 * time_step);

                let point_at_t = position(t);

                let velocity = motion.velocity_angular.cross(point_at_t);

                assert!(
                    (velocity - fd_velocity).length() <
                        tolerance(1e-6, 1e-3) * (1.0 + fd_velocity.length()),
                    "{:?} at t = {}: velocity {:?}, finite difference {:?}",
                    rotation_type, t, velocity, fd_velocity
                );
            }
        }
    }

    #[test]
    /// The angular velocity should be exact for a constant angular velocity, also when the body
    /// rotates a large angle during one time step. The new rotation angles are found by rotating the
    /// body around a fixed axis, and extracting the XYZ angles from the rotation matrix.
    fn constant_angular_velocity_is_exact() {
        let angular_velocity = SpatialVector::new(0.3, -0.8, 1.1);
        let time_step = 0.5;
        let old_rotation = SpatialVector::new(0.2, -0.4, 2.0);

        let rotation_angle = angular_velocity.length() * time_step;
        let axis = angular_velocity.normalize();

        // Columns of the new rotation matrix
        let columns = [SpatialVector::unit_x(), SpatialVector::unit_y(), SpatialVector::unit_z()]
            .map(|unit| unit.rotate(old_rotation, RotationType::XYZ).rotate_around_axis(rotation_angle, axis));

        let r = |row: usize, col: usize| columns[col][row];

        // For R = Rz Ry Rx: R20 = -sin(θ), R21 = cos(θ) sin(φ), R22 = cos(θ) cos(φ),
        // R10 = sin(ψ) cos(θ), R00 = cos(ψ) cos(θ)
        let new_rotation = SpatialVector::new(
            r(2, 1).atan2(r(2, 2)),
            (-r(2, 0)).asin(),
            r(1, 0).atan2(r(0, 0)),
        );

        let calculated = angular_velocity_from_rotation_change(
            old_rotation,
            new_rotation,
            time_step,
            RotationType::XYZ
        );

        assert!(
            (calculated - angular_velocity).length() < tolerance(1e-9, 1e-5),
            "calculated: {:?}, expected: {:?}", calculated, angular_velocity
        );
    }

    #[test]
    /// A rotation of close to π during one time step uses a separate branch, where the axis is
    /// found from the symmetric part of the rotation matrix
    fn rotation_of_half_a_turn() {
        let time_step = 1.0;
        let axis = SpatialVector::new(1.0, 2.0, -0.5).normalize();
        let old_rotation = SpatialVector::new(0.1, 0.2, 0.3);

        for angle in [PI - 1e-7, PI] {
            let columns = [SpatialVector::unit_x(), SpatialVector::unit_y(), SpatialVector::unit_z()]
                .map(|unit| unit.rotate(old_rotation, RotationType::XYZ).rotate_around_axis(angle, axis));

            let r = |row: usize, col: usize| columns[col][row];

            let new_rotation = SpatialVector::new(
                r(2, 1).atan2(r(2, 2)),
                (-r(2, 0)).asin(),
                r(1, 0).atan2(r(0, 0)),
            );

            let calculated = angular_velocity_from_rotation_change(
                old_rotation,
                new_rotation,
                time_step,
                RotationType::XYZ
            );

            // The sign of the axis is ambiguous for a rotation of exactly π
            assert!((calculated.length() - angle).abs() < 1e-5, "length: {}", calculated.length());
            assert!(calculated.normalize().cross(axis).length() < 1e-5);
        }
    }

    #[test]
    /// Test that the angular velocity is correct when the rotation angle passes +/- PI, in both
    /// directions.
    fn rotation_velocity_across_quadrant_switch() {
        let time_step = 0.1;
        let angle_change = 0.2;
        let allowable_error = 1e-4;

        let start_angles = [PI - 0.5 * angle_change, -PI + 0.5 * angle_change];
        let end_angles = [-PI + 0.5 * angle_change, PI - 0.5 * angle_change];
        let expected_velocities = [angle_change / time_step, -angle_change / time_step];

        for i in 0..2 {
            let mut motion = RigidBodyMotion {
                rotation: SpatialVector::from([0.0, 0.0, start_angles[i]]),
                ..Default::default()
            };

            motion.update_rotation_with_velocity_using_finite_difference(
                SpatialVector::from([0.0, 0.0, end_angles[i]]),
                time_step
            );

            let velocity = motion.velocity_angular[2];

            assert!(
                (velocity - expected_velocities[i]).abs() < allowable_error,
                "velocity: {}, expected: {}", velocity, expected_velocities[i]
            );
        }
    }
}
