use super::*;

impl SpatialVector {
    #[inline(always)]
    /// Returns the dot product of two vectors
    pub fn dot(self, rhs: Self) -> Float {
        self[0] * rhs[0] + self[1] * rhs[1] + self[2] * rhs[2]
    }

    #[inline(always)]
    /// Returns the length of the vector squared, which is equal to the dot product of the vector 
    /// with itself
    pub fn length_squared(self) -> Float {
        self[0] * self[0] + self[1] * self[1] + self[2] * self[2]
    }

    #[inline(always)]
    /// Returns the length of the vector
    pub fn length(self) -> Float {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    /// Returns a normalized version of the vector
    pub fn normalize(&self) -> Self {
        let length: Float = self.length();

        if length > 0.0 {
            let mut result = [0.0; DATA_SIZE];

            for i in 0..VECTOR_LENGTH {
                result[i] = self[i] / length;
            }

            Self(result)
        }
        else {
            Self::default()
        }
    }

    #[inline(always)]
    /// Returns the distance between two vectors
    pub fn distance(self, rhs: Self) -> Float {
        (self - rhs).length()
    }

    #[inline(always)]
    /// Returns the cross product of two vectors
    pub fn cross(self, rhs: Self) -> Self {
        let x = self[1] * rhs[2] - self[2] * rhs[1];
        let y = self[2] * rhs[0] - self[0] * rhs[2];
        let z = self[0] * rhs[1] - self[1] * rhs[0];
        
        Self::new(x, y, z)
    }

    #[inline(always)]
    /// Returns the absolute value of the angle between two vectors
    pub fn absolute_angle_between(self, rhs: Self) -> Float {
        let self_len_sq = self.length_squared();
        let rhs_len_sq = rhs.length_squared();

        if self_len_sq == 0.0 || rhs_len_sq == 0.0 {
            return 0.0;
        }

        let cosine_value = self.dot(rhs) / (self_len_sq * rhs_len_sq).sqrt();

        // Correct for potential floating point errors
        let clipped_cosine_value = cosine_value.max(-1.0).min(1.0);

        clipped_cosine_value.acos()
    }

    #[inline(always)]
    /// Returns the signed angle between two vectors, with the sign determined by the axis.
    /// 
    /// The sign is determined by the right-hand rule where the rotation is from self to rhs.
    pub fn signed_angle_between(self, rhs: Self, axis: Self) -> Float {
        let triple_product = self.dot(rhs.cross(axis));

        let absolute_angle = self.absolute_angle_between(rhs);

        if triple_product > 0.0 {
            absolute_angle
        } else {
            -absolute_angle
        }
    }

    #[inline(always)]
    /// Vector projection, as described in <https://en.wikipedia.org/wiki/Vector_projection>
    pub fn project(self, rhs: Self) -> Self {
        let rhs_normalized = rhs.normalize();

        rhs_normalized * self.dot(rhs_normalized)
    }

    #[inline(always)]
    /// Projects the vector onto a plane defined by the normal vector
    pub fn project_on_plane(self, plane_normal: Self) -> Self {
        self - self.project(plane_normal)
    }

}