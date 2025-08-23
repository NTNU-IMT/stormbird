// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::ops;

use super::*;

impl ops::Index<usize> for SpatialVector {
    type Output = Float;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0..VECTOR_LENGTH => &self.0[index],
            _ => panic!("SpatialVector index {} out of bounds (0-2)", index),
        }
    }
}

impl ops::IndexMut<usize> for SpatialVector {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0..VECTOR_LENGTH => &mut self.0[index], 
            _ => panic!("SpatialVector index {} out of bounds (0-2)", index),
        }
    }
}


impl ops::Add for SpatialVector {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut result = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            result[i] = self[i] + other.0[i];
        }

        Self(result)
    }
}

impl ops::AddAssign for SpatialVector {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..VECTOR_LENGTH {
            self.0[i] = self.0[i] + rhs.0[i];
        }
    }
}

impl ops::Sub for SpatialVector {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let mut result = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            result[i] = self[i] - other.0[i];
        }

        Self(result)
    }
}

impl ops::SubAssign for SpatialVector {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..VECTOR_LENGTH {
            self.0[i] = self.0[i] - rhs.0[i];
        }
    }
}

impl ops::Mul<Float> for SpatialVector {
    type Output = Self;

    fn mul(self, scalar: Float) -> Self {
        let mut result = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            result[i] = self[i] * scalar;
        }

        Self(result)
    }
}

impl ops::Mul<Self> for SpatialVector {
    type Output = Self;

    /// Element-wise multiplication of two vectors.
    fn mul(self, other: Self) -> Self {
        let mut result = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            result[i] = self[i] * other[i];
        }

        Self(result)
    }
}

impl ops::MulAssign<Float> for SpatialVector {
    fn mul_assign(&mut self, rhs: Float) {
        for i in 0..VECTOR_LENGTH {
            self.0[i] = self.0[i] * rhs;
        }
    }
}

impl ops::Mul<SpatialVector> for Float {
    type Output = SpatialVector;

    fn mul(self, rhs: SpatialVector) -> SpatialVector {
        let mut result = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            result[i] = rhs[i] * self;
        }

        SpatialVector(result)
    }
}

impl ops::Div<Float> for SpatialVector {
    type Output = Self;

    fn div(self, rhs: Float) -> Self {
        let mut result = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            result[i] = self[i] / rhs;
        }

        Self(result)
    }
}

impl ops::DivAssign<Float> for SpatialVector {
    fn div_assign(&mut self, rhs: Float) {
        for i in 0..VECTOR_LENGTH {
            self.0[i] = self.0[i] / rhs;
        }
    }
}

impl ops::Neg for SpatialVector {
    type Output = Self;

    fn neg(self) -> Self {
        let mut result = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            result[i] = -self[i];
        }

        Self(result)
    }
}