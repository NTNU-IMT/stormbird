// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Geometry transformations, such as rotation and translation

use crate::vec3::Vec3;

impl Vec3 {
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

        self * cos_angle + axis_normalized.cross(self) * sin_angle + axis_normalized * axis_normalized.dot(self) * (1.0 - cos_angle)   
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
    pub fn rotate(self, rotation: Self) -> Self {
        let mut new_vector = self;

        if rotation.x != 0.0 {
            let temp_y = new_vector.y;
            let temp_z = new_vector.z;
            new_vector.y = temp_y * rotation.x.cos() - temp_z * rotation.x.sin();
            new_vector.z = temp_y * rotation.x.sin() + temp_z * rotation.x.cos();
        }

        if rotation.y != 0.0 {
            let temp_x = new_vector.x;
            let temp_z = new_vector.z;
            new_vector.x = temp_x * rotation.y.cos() + temp_z * rotation.y.sin();
            new_vector.z = -temp_x * rotation.y.sin() + temp_z * rotation.y.cos();
        }

        if rotation.z != 0.0 {
            let temp_x = new_vector.x;
            let temp_y = new_vector.y;
            new_vector.x = temp_x * rotation.z.cos() - temp_y * rotation.z.sin();
            new_vector.y = temp_x * rotation.z.sin() + temp_y * rotation.z.cos();
        }

        new_vector
    }

    #[inline]
    /// Rotates the vector around a point in space
    pub fn rotate_around_point(self, rotation: Self, point: Self) -> Self {
        (self - point).rotate(rotation) + point
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// The spatial vector implements to different algorithms for rotations, with slightly different
    /// use cases. This test compares the two algorithms to ensure they produce the same result.
    fn compare_rotations() {

        let rotation = Vec3 {
            x: 30.0_f64.to_radians(),
            y: 21.2_f64.to_radians(),
            z: -16.1_f64.to_radians(),
        };

        let original_vector = Vec3::new(1.3, 1.2, 1.5);

        let rotated_vector_1 = original_vector.rotate(rotation);
        let rotated_vector_2 = original_vector
            .rotate_around_axis(rotation.x, Vec3::unit_x())
            .rotate_around_axis(rotation.y, Vec3::unit_y())
            .rotate_around_axis(rotation.z, Vec3::unit_z());

        dbg!(rotated_vector_1, rotated_vector_2);

        assert_eq!(rotated_vector_1, rotated_vector_2);
    }
}