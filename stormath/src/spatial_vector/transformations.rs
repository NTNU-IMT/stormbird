// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Geometry transformations, such as rotation and translation

use super::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Defines how the rotation angles in a rotation vector, [x, y, z], are applied.
///
/// The rotation vector always contains the angle around the x-axis, the angle around the y-axis
/// and the angle around the z-axis, in that order. The rotation type defines the order in which
/// the three rotations are applied. All rotations are around the *fixed* axes of the original
/// coordinate system (extrinsic rotations). For instance, `XYZ` first rotates around the x-axis,
/// then around the fixed y-axis, and finally around the fixed z-axis, which gives the rotation
/// matrix R = Rz Ry Rx.
///
/// A sequence of rotations around fixed axes is the same as the reversed sequence of rotations
/// around the rotated axes (intrinsic rotations). `XYZ` is therefore the same as a rotation around
/// the z-axis, then around the new y-axis, and then around the new x-axis, which is the yaw, pitch
/// and roll convention that is common for ships, when x points forward and z up or down.
pub enum RotationType {
    #[default]
    /// Rotation around the fixed x-axis, then the fixed y-axis, and then the fixed z-axis
    XYZ,
    /// Rotation around the fixed x-axis, then the fixed z-axis, and then the fixed y-axis
    XZY,
    /// Rotation around the fixed y-axis, then the fixed x-axis, and then the fixed z-axis
    YXZ,
    /// Rotation around the fixed y-axis, then the fixed z-axis, and then the fixed x-axis
    YZX,
    /// Rotation around the fixed z-axis, then the fixed x-axis, and then the fixed y-axis
    ZXY,
    /// Rotation around the fixed z-axis, then the fixed y-axis, and then the fixed x-axis
    ZYX,
}

impl RotationType {
    /// Returns the indices of the axes, where 0 is x, 1 is y and 2 is z, in the order the
    /// rotations are applied
    pub fn axis_order(self) -> [usize; 3] {
        match self {
            RotationType::XYZ => [0, 1, 2],
            RotationType::XZY => [0, 2, 1],
            RotationType::YXZ => [1, 0, 2],
            RotationType::YZX => [1, 2, 0],
            RotationType::ZXY => [2, 0, 1],
            RotationType::ZYX => [2, 1, 0],
        }
    }
}

impl SpatialVector {
    #[inline(always)]
    /// Translates the vector by the given translation vector
    pub fn translate(self, translation: Self) -> Self {
        self + translation
    }

    #[inline(always)]
    /// Uses the Rodrigues' rotation formula to rotate the vector around the given axis.
    /// 
    /// Source: <https://en.wikipedia.org/wiki/Rodrigues%27_rotation_formula>
    pub fn rotate_around_axis(self, angle: Float, axis: Self) -> Self {
        let axis_normalized = axis.normalize();

        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        let term1 = self * cos_angle;
        let term2 = axis_normalized.cross(self) * sin_angle;
        let term3 = axis_normalized * axis_normalized.dot(self) * (1.0 - cos_angle);

        term1 + term2 + term3
    }

    /// Rotates the vector with the given rotation angles and rotation type
    pub fn rotate(self, rotation: Self, rotation_type: RotationType) -> Self {
        let mut new_vector = self;

        for axis_index in rotation_type.axis_order() {
            new_vector = new_vector.rotate_around_coordinate_axis(axis_index, rotation[axis_index]);
        }

        new_vector
    }

    #[inline(always)]
    /// Rotates the vector around one of the axes in the coordinate system, where 0 is the x-axis,
    /// 1 is the y-axis and 2 is the z-axis. The rotation follows the right-hand rule.
    pub fn rotate_around_coordinate_axis(self, axis_index: usize, angle: Float) -> Self {
        if angle == 0.0 {
            return self;
        }

        // The two other axes, in the order that gives a right-handed rotation
        let (first, second) = match axis_index {
            0 => (1, 2),
            1 => (2, 0),
            2 => (0, 1),
            _ => panic!("The axis index must be 0, 1 or 2, but is {}", axis_index),
        };

        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        let mut new_vector = self;

        new_vector[first] = self[first] * cos_angle - self[second] * sin_angle;
        new_vector[second] = self[first] * sin_angle + self[second] * cos_angle;

        new_vector
    }

    #[inline(always)]
    /// Performs rotation of the vector based on Euler angles specified in the rotation vector
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
        self.rotate(rotation, RotationType::XYZ)
    }

    #[inline(always)]
    /// Rotates the vector around a point in space
    pub fn rotate_around_point(self, rotation: Self, point: Self, rotation_type: RotationType) -> Self {
        (self - point).rotate(rotation, rotation_type) + point
    }

    #[inline(always)]
    /// Returns the x, y and z axes of a coordinate system rotated by the given rotation
    pub fn rotated_system_axes(&self, rotation: Self, rotation_type: RotationType) -> [Self; 3] {
        let x_axis = SpatialVector::unit_x().rotate(rotation, rotation_type);
        let y_axis = SpatialVector::unit_y().rotate(rotation, rotation_type);
        let z_axis = SpatialVector::unit_z().rotate(rotation, rotation_type);

        [x_axis, y_axis, z_axis]
    }

    #[inline(always)]
    /// Assumes self is expressed in a rotated coordinate system, specified by the system rotation
    /// input, and transforms it back to the global coordinate system.
    pub fn from_rotated_to_global_system(self, system_rotation: Self, rotation_type: RotationType) -> Self {
        let rotated_system_axes = self.rotated_system_axes(system_rotation, rotation_type);

        Self::from([
            self[0] * rotated_system_axes[0][0] + self[1] * rotated_system_axes[1][0] + self[2] * rotated_system_axes[2][0],
            self[0] * rotated_system_axes[0][1] + self[1] * rotated_system_axes[1][1] + self[2] * rotated_system_axes[2][1],
            self[0] * rotated_system_axes[0][2] + self[1] * rotated_system_axes[1][2] + self[2] * rotated_system_axes[2][2],
        ])
    }

    #[inline(always)]
    /// Expresses the vector in a coordinate system that is rotated by the input rotation, relative
    /// to the original coordinate system for the vector.
    pub fn in_rotated_coordinate_system(self, system_rotation: Self, rotation_type: RotationType  ) -> Self {
        let rotated_system_axes = self.rotated_system_axes(system_rotation, rotation_type);

        Self::from([
            self.dot(rotated_system_axes[0]),
            self.dot(rotated_system_axes[1]),
            self.dot(rotated_system_axes[2]),
        ])
    }

    #[inline(always)]
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
        let rotation = SpatialVector::new(
            Float::to_radians(30.0),
            Float::to_radians(21.2),
            Float::to_radians(-16.1),
        );

        let rotation_type = RotationType::XYZ;

        let original_vector = SpatialVector::new(1.3, 1.2, 1.5);

        let rotated_vector_1 = original_vector.rotate(rotation, rotation_type);
        let rotated_vector_2 = original_vector
            .rotate_around_axis(rotation[0], SpatialVector::unit_x())
            .rotate_around_axis(rotation[1], SpatialVector::unit_y())
            .rotate_around_axis(rotation[2], SpatialVector::unit_z());

        dbg!(rotated_vector_1, rotated_vector_2);

        assert_eq!(rotated_vector_1, rotated_vector_2);
    }

    #[test]
    fn moment_transformation() {
        let rotation = SpatialVector::default();
        let location = SpatialVector::from([1.2, 0.0, 0.0]);
        let force = SpatialVector::from([0.0, 2.0, 0.0]);

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
        let vector = SpatialVector::from([-1.0, 0.0, 0.0]);

        let negative_axis = SpatialVector::from([0.0, 0.0, -1.0]);
        let positive_axis = SpatialVector::from([0.0, 0.0, 1.0]);

        let rotation_angle = Float::to_radians(90.0);

        let rotated_vector_1 = vector.rotate_around_axis(-rotation_angle, negative_axis);
        let rotated_vector_2 = vector.rotate_around_axis(rotation_angle, positive_axis);

        let difference = rotated_vector_1 - rotated_vector_2;

        assert!(difference[0].abs() < 1e-6);
        assert!(difference[1].abs() < 1e-6);
        assert!(difference[2].abs() < 1e-6);
    }

    #[test]
    fn xyz_rotation_compared_against_blender(){
        let vector = SpatialVector::from([1.0, 1.0, 1.0]);

        let rotation = SpatialVector::from(
            [
                Float::to_radians(30.0),
                Float::to_radians(21.2),
                Float::to_radians(-16.1),
            ]
        );

        let expected_result = SpatialVector::from([1.47188, -0.043868, 0.91195]); // From blender

        let calculated_result = vector.rotate_xyz(rotation);

        assert!((calculated_result[0] - expected_result[0]).abs() < 0.00001);
        assert!((calculated_result[1] - expected_result[1]).abs() < 0.00001);
        assert!((calculated_result[2] - expected_result[2]).abs() < 0.00001);
        

        dbg!(calculated_result, expected_result);
    }

    #[test]
    /// Compares all rotation types against rotations around each fixed axis with Rodrigues'
    /// formula, applied in the order given by the rotation type
    fn all_rotation_types_follow_the_axis_order() {
        let rotation = SpatialVector::new(
            Float::to_radians(30.0),
            Float::to_radians(21.2),
            Float::to_radians(-16.1),
        );

        let axes = [SpatialVector::unit_x(), SpatialVector::unit_y(), SpatialVector::unit_z()];

        let original_vector = SpatialVector::new(1.3, 1.2, 1.5);

        let rotation_types = [
            RotationType::XYZ, RotationType::XZY, RotationType::YXZ,
            RotationType::YZX, RotationType::ZXY, RotationType::ZYX,
        ];

        let mut results: Vec<SpatialVector> = Vec::new();

        for rotation_type in rotation_types {
            let mut expected = original_vector;

            for axis_index in rotation_type.axis_order() {
                expected = expected.rotate_around_axis(rotation[axis_index], axes[axis_index]);
            }

            let calculated = original_vector.rotate(rotation, rotation_type);

            assert!(
                (calculated - expected).length() < 1e-6,
                "{:?}: calculated {:?}, expected {:?}", rotation_type, calculated, expected
            );

            results.push(calculated);
        }

        // The rotation types should give different results for a general rotation
        for i in 0..results.len() {
            for j in (i + 1)..results.len() {
                assert!((results[i] - results[j]).length() > 1e-3);
            }
        }
    }

    #[test]
    /// The XYZ rotation around fixed axes should be the same as yaw, pitch and roll around the
    /// rotated axes
    fn xyz_rotation_equals_intrinsic_yaw_pitch_roll() {
        let vector = SpatialVector::from([1.2, 4.0, 2.0]);

        let roll_angle = Float::to_radians(-16.1);
        let pitch_angle = Float::to_radians(21.2);
        let yaw_angle = Float::to_radians(30.0);

        let yaw_axis = SpatialVector::unit_z();
        let pitch_axis = SpatialVector::unit_y().rotate_around_axis(yaw_angle, yaw_axis);
        let roll_axis = SpatialVector::unit_x()
            .rotate_around_axis(yaw_angle, yaw_axis)
            .rotate_around_axis(pitch_angle, pitch_axis);

        let intrinsic = vector
            .rotate_around_axis(yaw_angle, yaw_axis)
            .rotate_around_axis(pitch_angle, pitch_axis)
            .rotate_around_axis(roll_angle, roll_axis);

        let extrinsic = vector.rotate(
            SpatialVector::from([roll_angle, pitch_angle, yaw_angle]),
            RotationType::XYZ
        );

        assert!((intrinsic - extrinsic).length() < 1e-6);
    }

    #[test]
    fn rotation_types_from_json() {
        let rotation_type: RotationType = serde_json::from_str("\"ZYX\"").unwrap();

        assert_eq!(rotation_type, RotationType::ZYX);
        assert!(serde_json::from_str::<RotationType>("\"YawPitchRoll\"").is_err());
    }
}