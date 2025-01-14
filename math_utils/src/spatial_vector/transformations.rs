// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Geometry transformations, such as rotation and translation

use super::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RotationType {
    #[default]
    XYZ,
    YawPitchRoll,
}

impl SpatialVector<3> {
    #[inline]
    /// Translates the vector by the given translation vector
    pub fn translate(self, translation: Self) -> Self {
        self + translation
    }

    #[inline]
    /// Uses the Rodrigues' rotation formmula to rotate the vector around the given axis.
    /// 
    /// Source: <https://en.wikipedia.org/wiki/Rodrigues%27_rotation_formula>
    pub fn rotate_around_axis(self, angle: f64, axis: Self) -> Self {
        let axis_normalized = axis.normalize();

        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        let term1 = self * cos_angle;
        let term2 = axis_normalized.cross(self) * sin_angle;
        let term3 = axis_normalized * axis_normalized.dot(self) * (1.0 - cos_angle);

        term1 + term2 + term3
    }

    pub fn rotate(self, rotation: Self, rotation_type: RotationType) -> Self {
        match rotation_type {
            RotationType::XYZ => self.rotate_xyz(rotation),
            RotationType::YawPitchRoll => self.rotate_yaw_pitch_roll(rotation),
        }
    }

    #[inline]
    /// performs rotation of the vector based on Euler angles specified in the rotation vector
    /// 
    /// # Operation
    /// mut New vector = old vector * Rx
    /// new vector     = new vector * Ry
    /// new vector     = new vector * Rz
    /// 
    /// # Rotation matrices
    /// ## Rx
    /// | 1 0    0   |
    /// | 0 cos -sin |
    /// | 0 sin  cos |
    /// 
    /// ## Ry
    /// |  cos 0 sin |
    /// |  0   1 0   |
    /// | -sin 0 cos |
    /// 
    /// ## Rz
    /// | cos -sin 0 |
    /// | sin  cos 0 |
    /// | 0    0   1 |
    pub fn rotate_xyz(self, rotation: Self) -> Self {
        let mut new_vector = self;

        if rotation[0] != 0.0 {
            let temp_y = new_vector[1];
            let temp_z = new_vector[2];
            new_vector[1] = temp_y * rotation[0].cos() - temp_z * rotation[0].sin();
            new_vector[2] = temp_y * rotation[0].sin() + temp_z * rotation[0].cos();
        }

        if rotation[1] != 0.0 {
            let temp_x = new_vector[0];
            let temp_z = new_vector[2];
            new_vector[0] = temp_x * rotation[1].cos() + temp_z * rotation[1].sin();
            new_vector[2] = -temp_x * rotation[1].sin() + temp_z * rotation[1].cos();
        }

        if rotation[2] != 0.0 {
            let temp_x = new_vector[0];
            let temp_y = new_vector[1];
            new_vector[0] = temp_x * rotation[2].cos() - temp_y * rotation[2].sin();
            new_vector[1] = temp_x * rotation[2].sin() + temp_y * rotation[2].cos();
        }

        new_vector
    }

    #[inline]
    /// Rotates the vector around the yaw, pitch, and roll angles specified in the rotation vector
    pub fn rotate_yaw_pitch_roll(self, rotation: Self) -> Self {
        let yaw_axis = SpatialVector::<3>::unit_z();

        let pitch_axis = SpatialVector::<3>::unit_y()
            .rotate_around_axis(rotation[2], yaw_axis);

        let roll_axis = SpatialVector::<3>::unit_x()
            .rotate_around_axis(rotation[2], yaw_axis)
            .rotate_around_axis(rotation[1], pitch_axis);

        self.rotate_around_axis(rotation[2], yaw_axis)
            .rotate_around_axis(rotation[1], pitch_axis)
            .rotate_around_axis(rotation[0], roll_axis)
    }

    #[inline]
    /// Rotates the vector around a point in space
    pub fn rotate_around_point(self, rotation: Self, point: Self, rotation_type: RotationType) -> Self {
        (self - point).rotate(rotation, rotation_type) + point
    }

    /// Assumes self is expressed in a rotated coordinate system, specified by the system rotation
    /// input, and transforms it back to the global coordinate system.
    pub fn from_rotated_to_global_system(self, system_rotation: Self, rotation_type: RotationType) -> Self {
        let rotated_x_axis = SpatialVector::<3>::unit_x().rotate(system_rotation, rotation_type);
        let rotated_y_axis = SpatialVector::<3>::unit_y().rotate(system_rotation, rotation_type);
        let rotated_z_axis = SpatialVector::<3>::unit_z().rotate(system_rotation, rotation_type);

        Self([
            self[0] * rotated_x_axis[0] + self[1] * rotated_y_axis[0] + self[2] * rotated_z_axis[0],
            self[0] * rotated_x_axis[1] + self[1] * rotated_y_axis[1] + self[2] * rotated_z_axis[1],
            self[0] * rotated_x_axis[2] + self[1] * rotated_y_axis[2] + self[2] * rotated_z_axis[2],
        ])
    }

    /// Expresses the vector in a coordinate system that is rotated by the input rotation, relative
    /// to the original coordinate system for the vector.
    pub fn in_rotated_coordinate_system(self, system_rotation: Self, rotation_type: RotationType  ) -> Self {
        let x_axis = SpatialVector::<3>::unit_x().rotate(system_rotation, rotation_type);
        let y_axis = SpatialVector::<3>::unit_y().rotate(system_rotation, rotation_type);
        let z_axis = SpatialVector::<3>::unit_z().rotate(system_rotation, rotation_type);

        Self([
            self.dot(x_axis),
            self.dot(y_axis),
            self.dot(z_axis),
        ])
    }

    /// Special function for transforming a vector representing a moment to a new coordinate system.
    /// 
    /// The transformation consists of two steps:
    /// 1. Rotate the moment vector to the new coordinate system
    /// 2. Add the moment created by the force in the new coordinate system
    pub fn moment_in_new_coordinate_system(
        self, 
        system_rotation: Self,
        system_translation: Self,
        force_that_created_the_moment: Self,
        rotation_type: RotationType
    ) -> Self {
        // Rotation of the moment vector
        let rotated_moment = self.in_rotated_coordinate_system(system_rotation, rotation_type);
        
        // The additional moment created by the force due to translation
        // The *arm* in the new coordinate system is the negative of the translation vector from old
        // to new coordinate system. The force is the same in both coordinate systems. However, the
        // resulting moment must also be rotated.
        let moment_arm = -system_translation;
        let moment_due_to_force = moment_arm.cross(force_that_created_the_moment);
        let rotated_moment_due_to_force = moment_due_to_force.in_rotated_coordinate_system(system_rotation, rotation_type);

        rotated_moment + rotated_moment_due_to_force
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// The spatial vector implements to different algorithms for rotations, with slightly different
    /// use cases. This test compares the two algorithms to ensure they produce the same result.
    fn compare_rotations() {
        let rotation = SpatialVector::<3>::new(
            30.0_f64.to_radians(),
            21.2_f64.to_radians(),
            -16.1_f64.to_radians(),
        );

        let rotation_type = RotationType::XYZ;

        let original_vector = SpatialVector::<3>::new(1.3, 1.2, 1.5);

        let rotated_vector_1 = original_vector.rotate(rotation, rotation_type);
        let rotated_vector_2 = original_vector
            .rotate_around_axis(rotation[0], SpatialVector::<3>::unit_x())
            .rotate_around_axis(rotation[1], SpatialVector::<3>::unit_y())
            .rotate_around_axis(rotation[2], SpatialVector::<3>::unit_z());

        dbg!(rotated_vector_1, rotated_vector_2);

        assert_eq!(rotated_vector_1, rotated_vector_2);
    }

    #[test]
    fn moment_transformation() {
        let rotation = SpatialVector::<3>::default();
        let location = SpatialVector([1.2, 0.0, 0.0]);
        let force = SpatialVector([0.0, 2.0, 0.0]);

        let moment = location.cross(force);

        let rotation_type = RotationType::XYZ;

        let transformed_moment = moment.moment_in_new_coordinate_system(
            rotation,
            location,
            force,
            rotation_type
        );

        dbg!(moment, transformed_moment);

        assert!(transformed_moment.length() < 1e-6)
    }

    #[test]
    fn negative_rotation() {
        let vector = SpatialVector([-1.0, 0.0, 0.0]);
        
        let negative_axis = SpatialVector([0.0, 0.0, -1.0]);
        let positive_axis = SpatialVector([0.0, 0.0, 1.0]);

        let rotation_angle = 90.0_f64.to_radians();

        let rotated_vector_1 = vector.rotate_around_axis(-rotation_angle, negative_axis);
        let rotated_vector_2 = vector.rotate_around_axis(rotation_angle, positive_axis);
        let rotated_vector_3 = vector.rotate_around_axis(rotation_angle, negative_axis);

        dbg!(rotated_vector_1, rotated_vector_2, rotated_vector_3);
    }
}