// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation blocks with ways to transform a matrix. Includes the following:
//!
//! - [`transpose`](Matrix::transpose)
//! - [`diagonal`](Matrix::diagonal)

use super::*;

impl<T> Matrix<T>
where T: Default + Clone + Copy + Debug,
{
    /// Transposes the matrix, swapping rows and columns.
    pub fn transpose(&self) -> Self {
        let mut result = Matrix::new_default([self.shape[1], self.shape[0]]);

        for i in 0..self.shape[0] {
            for j in 0..self.shape[1] {
                result[[j, i]] = self[[i, j]];
            }
        }

        result
    }

    /// Returns a new matrix that only contains the diagonal of self
    pub fn diagonal(&self) -> Self {
        assert!(self.shape[0] == self.shape[1], "Matrix must be square to extract diagonal");

        let mut result = Matrix::new_default(self.shape);

        for i in 0..self.shape[0] {
            result[[i, i]] = self[[i, i]];
        }

        result
    }
}
