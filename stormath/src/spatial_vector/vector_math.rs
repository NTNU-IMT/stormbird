use super::*;

impl<const N: usize> SpatialVector<N> {
    #[inline]
    /// Returns the dot product of two vectors
    pub fn dot(self, rhs: Self) -> f64 {
        let mut result = 0.0;

        for i in 0..N {
            result += self[i] * rhs[i];
        }

        result
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
            let mut result = [0.0; N];

            for i in 0..N {
                result[i] = self[i] / length;
            }

            Self(result)
        }
        else {
            Self::default()
        }
    }

    #[inline]
    /// Returns the distance between two vectors
    pub fn distance(self, rhs: Self) -> f64 {
        (self - rhs).length()
    }

}

impl SpatialVector<3> {
    #[inline]
    /// Returns the cross product of two vectors
    pub fn cross(self, rhs: Self) -> Self {
        let x = self[1] * rhs[2] - self[2] * rhs[1];
        let y = self[2] * rhs[0] - self[0] * rhs[2];
        let z = self[0] * rhs[1] - self[1] * rhs[0];
        
        Self::new(x, y, z)
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