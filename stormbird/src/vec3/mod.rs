// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! 3D spatial vectors and related functions

use serde::{Serialize, Deserialize};

pub mod iterators;
pub mod operators;
pub mod transformations;
pub mod geometry_functions;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A 3D vector with typical geometric functions implemented
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    /// Creates a new vector
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {x, y, z}
    }

    /// Returns a vector that has length 1 and oriented along the the x-axis
    pub fn unit_x() -> Self {
        Self {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Returns a vector that has length 1 and oriented along the the y-axis
    pub fn unit_y() -> Self {
        Self {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }

    /// Returns a vector that has length 1 and oriented along the the z-axis
    pub fn unit_z() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    }

    #[inline]
    /// Returns the cross product of two vectors
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    #[inline]
    /// Returns the dot product of two vectors
    pub fn dot(self, rhs: Self) -> f64 {
        (self.x * rhs.x) + (self.y * rhs.y) + (self.z * rhs.z)
    }

    #[inline]
    /// Returns the length of the vector squared, which is equal to the dot product of the vector 
    /// with itself
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    #[inline]
    /// Returns the length of the vector
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    #[inline]
    /// Returns a normalized version of the vector
    pub fn normalize(&self) -> Self {
        let length: f64 = self.length();

        if length > 0.0 {
            return Self {
                x: self.x / length,
                y: self.y / length,
                z: self.z / length,
            };
        }
        else {
            return Self::default();
        }
    }

    #[inline]
    /// Returns the distance between two vectors
    pub fn distance(self, rhs: Self) -> f64 {
        (self - rhs).length()
    }

    #[inline]
    /// Returns the absolute value of the angle between two vectors
    pub fn absolute_angle_between(self, rhs: Self) -> f64 {
        if self.length() == 0.0 || rhs.length() == 0.0 {
            return 0.0;
        }

        let cosine_value = self.dot(rhs) / (self.length() * rhs.length());

        // Correct for potential floating point errors
        let clipped_cosine_value = cosine_value.max(-1.0).min(1.0);
        
        clipped_cosine_value.acos()
    }

    #[inline]
    /// Returns the signed angle between two vectors, with the sign determined by the axis.
    /// 
    /// The sign is determined by the right-hand rule where the rotation is from self to rhs.
    pub fn signed_angle_between(self, rhs: Self, axis: Self) -> f64 {
        let triple_product = self.dot(rhs.cross(axis));

        let absolute_angle = self.absolute_angle_between(rhs);

        if triple_product > 0.0 {
            absolute_angle
        } else {
            -absolute_angle
        }
    }

    #[inline]
    /// Vector projection, as described in <https://en.wikipedia.org/wiki/Vector_projection>
    pub fn project(self, rhs: Self) -> Self {
        let rhs_normalized = rhs.normalize();

        self.dot(rhs_normalized) * rhs_normalized
    }

    #[inline]
    /// Projects the vector onto a plane defined by the normal vector
    pub fn project_on_plane(self, plane_normal: Self) -> Self {
        self - self.project(plane_normal)
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Tests the sign of the angle between two vectors. The angle is supposed to follow the 
    /// right-hand rule
    fn test_signed_angle() {
        let vector_1 = Vec3::new(1.2, 0.0, 0.0);

        let vector_2 = Vec3::new(1.2, -0.2, 0.0);

        let axis = Vec3::unit_z();

        let angle_from_1_to_2 = vector_1.signed_angle_between(vector_2, axis);
        let angle_from_2_to_1 = vector_2.signed_angle_between(vector_1, axis);
        let angle_from_1_to_2_neg_axis = vector_1.signed_angle_between(vector_2, -axis);

        dbg!(
            angle_from_1_to_2, 
            angle_from_2_to_1, 
            angle_from_1_to_2_neg_axis
        );

        assert!(angle_from_1_to_2 < 0.0);
        assert!(angle_from_2_to_1 > 0.0);
        assert!(angle_from_1_to_2_neg_axis > 0.0);
    }
}