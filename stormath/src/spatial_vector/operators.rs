// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::ops;

use super::*;

impl ops::Index<usize> for SpatialVector {
    type Output = f64;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0..3 => &self.0[index],
            _ => panic!("SpatialVector index {} out of bounds (0-2)", index),
        }
    }
}

impl ops::IndexMut<usize> for SpatialVector {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0..3 => &mut self.0[index], // or 0..=2 if you only want 3D access  
            _ => panic!("SpatialVector index {} out of bounds (0-2)", index),
        }
    }
}


impl ops::Add for SpatialVector {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut result = [0.0; 4];

        for i in 0..3 {
            result[i] = self[i] + other.0[i];
        }

        Self(result)
    }
}

impl ops::AddAssign for SpatialVector {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..3 {
            self.0[i] += rhs.0[i];
        }
    }
}

impl ops::Sub for SpatialVector {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let mut result = [0.0; 4];

        for i in 0..3 {
            result[i] = self[i] - other.0[i];
        }

        Self(result)
    }
}

impl ops::SubAssign for SpatialVector {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..3 {
            self.0[i] -= rhs.0[i];
        }
    }
}

impl ops::Mul<f64> for SpatialVector {
    type Output = Self;

    fn mul(self, scalar: f64) -> Self {
        let mut result = [0.0; 4];

        for i in 0..3 {
            result[i] = self[i] * scalar;
        }

        Self(result)
    }
}

impl ops::Mul<Self> for SpatialVector {
    type Output = Self;

    /// Element-wise multiplication of two vectors.
    fn mul(self, other: Self) -> Self {
        let mut result = [0.0; 4];

        for i in 0..3 {
            result[i] = self[i] * other[i];
        }

        Self(result)
    }
}

impl ops::MulAssign<f64> for SpatialVector {
    fn mul_assign(&mut self, rhs: f64) {
        for i in 0..3 {
            self.0[i] *= rhs;
        }
    }
}

impl ops::Mul<SpatialVector> for f64 {
    type Output = SpatialVector;

    fn mul(self, rhs: SpatialVector) -> SpatialVector {
        let mut result = [0.0; 4];

        for i in 0..3 {
            result[i] = self * rhs[i];
        }

        SpatialVector(result)
    }
}

impl ops::Div<f64> for SpatialVector {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        let mut result = [0.0; 4];

        for i in 0..3 {
            result[i] = self[i] / rhs;
        }

        Self(result)
    }
}

impl ops::DivAssign<f64> for SpatialVector {
    fn div_assign(&mut self, rhs: f64) {
        for i in 0..3 {
            self.0[i] /= rhs;
        }
    }
}

impl ops::Neg for SpatialVector {
    type Output = Self;

    fn neg(self) -> Self {
        let mut result = [0.0; 4];

        for i in 0..3 {
            result[i] = -self[i];
        }

        Self(result)
    }
}