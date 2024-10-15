// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Geometry transformations, such as rotation and translation

use super::*;

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
    /// Rotates the vector around a point in space
    pub fn rotate_around_point(self, rotation: Self, point: Self) -> Self {
        (self - point).rotate(rotation) + point
    }

    /// Expresses the vector in a coordinate system that is rotated by the input rotation, relative
    /// to the original coordinate system for the vector.
    pub fn in_rotated_coordinate_system(self, system_rotation: Self) -> Self {
        let x_axis = SpatialVector::<3>::unit_x().rotate(system_rotation);
        let y_axis = SpatialVector::<3>::unit_y().rotate(system_rotation);
        let z_axis = SpatialVector::<3>::unit_z().rotate(system_rotation);

        Self([
            self.dot(x_axis),
            self.dot(y_axis),
            self.dot(z_axis),
        ])
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

        let original_vector = SpatialVector::<3>::new(1.3, 1.2, 1.5);

        let rotated_vector_1 = original_vector.rotate(rotation);
        let rotated_vector_2 = original_vector
            .rotate_around_axis(rotation[0], SpatialVector::<3>::unit_x())
            .rotate_around_axis(rotation[1], SpatialVector::<3>::unit_y())
            .rotate_around_axis(rotation[2], SpatialVector::<3>::unit_z());

        dbg!(rotated_vector_1, rotated_vector_2);

        assert_eq!(rotated_vector_1, rotated_vector_2);
    }
}