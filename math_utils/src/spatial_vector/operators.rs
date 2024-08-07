// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::ops;

use super::*;

impl<const N: usize> ops::Index<usize> for SpatialVector<N> {
    type Output = f64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<const N: usize> ops::IndexMut<usize> for SpatialVector<N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}


impl<const N: usize> ops::Add for SpatialVector<N> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut result = [0.0; N];

        for i in 0..N {
            result[i] = self[i] + other.0[i];
        }

        Self(result)
    }
}

impl<const N: usize> ops::AddAssign for SpatialVector<N> {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..N {
            self.0[i] += rhs.0[i];
        }
    }
}

impl<const N: usize> ops::Sub for SpatialVector<N> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let mut result = [0.0; N];

        for i in 0..N {
            result[i] = self[i] - other.0[i];
        }

        Self(result)
    }
}

impl<const N: usize> ops::SubAssign for SpatialVector<N> {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..N {
            self.0[i] -= rhs.0[i];
        }
    }
}

impl<const N: usize> ops::Mul<f64> for SpatialVector<N> {
    type Output = Self;

    fn mul(self, scalar: f64) -> Self {
        let mut result = [0.0; N];

        for i in 0..N {
            result[i] = self[i] * scalar;
        }

        Self(result)
    }
}

impl<const N: usize> ops::MulAssign<f64> for SpatialVector<N> {
    fn mul_assign(&mut self, rhs: f64) {
        for i in 0..N {
            self.0[i] *= rhs;
        }
    }
}

impl<const N: usize> ops::Mul<SpatialVector<N>> for f64 {
    type Output = SpatialVector<N>;

    fn mul(self, rhs: SpatialVector<N>) -> SpatialVector<N> {
        let mut result = [0.0; N];

        for i in 0..N {
            result[i] = self * rhs[i];
        }

        SpatialVector(result)
    }
}

impl<const N: usize> ops::Div<f64> for SpatialVector<N> {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        let mut result = [0.0; N];

        for i in 0..N {
            result[i] = self[i] / rhs;
        }

        Self(result)
    }
}

impl<const N: usize> ops::DivAssign<f64> for SpatialVector<N> {
    fn div_assign(&mut self, rhs: f64) {
        for i in 0..N {
            self.0[i] /= rhs;
        }
    }
}

impl<const N: usize> ops::Neg for SpatialVector<N> {
    type Output = Self;

    fn neg(self) -> Self {
        let mut result = [0.0; N];

        for i in 0..N {
            result[i] = -self[i];
        }

        Self(result)
    }
}