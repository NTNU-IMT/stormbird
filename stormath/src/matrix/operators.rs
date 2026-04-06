
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! Implementation block for matrix operators. This includes the follwing:
//!
//! - General add, subtract and multiply
//! - Indexing into the matrix
//! - Special functions for [matrix multiplication](Matrix::matrix_multiply) and
//! [matrix and vector multiplication](Matrix::vector_multiply)

use std::ops::{Index, IndexMut, Add, Sub, Mul};

use super::*;

impl Index<[usize; 2]> for Matrix {
    type Output = Float;

    fn index(&self, indices: [usize; 2]) -> &Self::Output {
        let flat_index = self.flat_index(indices);
        &self.data[flat_index]
    }
}

impl IndexMut<[usize; 2]> for Matrix {
    fn index_mut(&mut self, indices: [usize; 2]) -> &mut Self::Output {
        let flat_index = self.flat_index(indices);

        &mut self.data[flat_index]
    }
}

impl Add for Matrix {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        assert_eq!(self.shape, other.shape, "Shapes do not match");

        let mut result = self.clone();

        for i in 0..self.data.len() {
            result.data[i] = self.data[i] + other.data[i];
        }

        result
    }
}

impl Sub for Matrix {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        assert_eq!(self.shape, other.shape, "Shapes do not match");

        let mut result = self.clone();

        for i in 0..self.data.len() {
            result.data[i] = self.data[i] - other.data[i];
        }

        result
    }
}

impl Mul<Float> for Matrix {
    type Output = Self;

    fn mul(self, scalar: Float) -> Self {
        let mut result = self.clone();

        for i in 0..self.data.len() {
            result.data[i] = self.data[i] * scalar;
        }

        result
    }
}

impl Mul<Matrix> for Float {
    type Output = Matrix;

    fn mul(self, matrix: Matrix) -> Matrix {
        let mut result = matrix.clone();

        for i in 0..matrix.data.len() {
            result.data[i] = matrix.data[i] * self;
        }

        result
    }
}


impl Matrix {
    /// Performs a matrix multiplication with the supplied matrix
    pub fn matrix_multiply(&self, other: &Matrix) -> Matrix {
        assert_eq!(self.shape[1], other.shape[0], "Matrix shapes do not match for multiplication");

        let mut result = Matrix::new_default([self.shape[0], other.shape[1]]);

        for i in 0..self.shape[0] {
            for j in 0..other.shape[1] {
                let mut sum = 0.0;
                for k in 0..self.shape[1] {
                    sum = sum + self[[i, k]] * other[[k, j]];
                }
                result[[i, j]] = sum;
            }
        }

        result
    }

    /// Performs a multiplication with the supplied vector
    pub fn vector_multiply(&self, vector: &[Float]) -> Vec<Float> {
        assert_eq!(
            self.shape[1], 
            vector.len(), 
            "Matrix and vector shapes do not match for multiplication"
        );

        let mut result = vec![0.0; self.shape[0]];

        for i in 0..self.shape[0] {
            let mut sum = 0.0;
            for j in 0..self.shape[1] {
                sum = sum + self[[i, j]] * vector[j];
            }
            result[i] = sum;
        }

        result
    }
}
