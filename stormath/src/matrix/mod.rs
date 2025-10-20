// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A matrix structure that comes with the standard operations for matrices

use std::fmt::Debug;
use serde::{Serialize, Deserialize};

pub mod linalg;
pub mod tranforms;
pub mod operators;

use crate::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A 2D matrix structure that can store data of any type `T`.
pub struct Matrix<T> {
    /// The data stored in the matrix, represented as a flat vector. The data is stored in
    /// row-major order.
    pub data: Vec<T>,
    /// The shape of the matrix, represented as a tuple of two usize values (rows, columns).
    pub shape: [usize; 2],
}

impl<T> Matrix<T>
where T: Default + Clone + Copy + Debug,
{
    /// Creates a new matrix with the specified shape where all values are equal to the supplied
    /// element value
    pub fn from_elem(shape: [usize; 2], elem: T) -> Self {
        let data = vec![elem; shape[0] * shape[1]];

        Self {
            data,
            shape
        }
    }

    /// New with default values. The value of the default depends on type. Zero would be the
    /// case for the Float type.
    pub fn new_default(shape: [usize; 2]) -> Self {
        Self::from_elem(shape, T::default())
    }

    #[inline(always)]
    /// Convert two-dimensional indices (row, column) to a flat index
    pub fn flat_index(&self, indices: [usize; 2]) -> usize {
        indices[0] * self.shape[1] + indices[1]
    }

    #[inline(always)]
    /// Converts a flat index to two-dimensional indices (row, column)
    pub fn indices_from_index(&self, flat_index: usize) -> [usize; 2] {
        [flat_index / self.shape[1], flat_index % self.shape[1]]
    }

    /// Returns the number of rows
    pub fn nr_rows(&self) -> usize {
        self.shape[0]
    }

    /// Returns the number of columns
    pub fn nr_cols(&self) -> usize {
        self.shape[1]
    }
}

impl Matrix<Float> {
    /// Creates a matrix with values of 1.0 along the diagonal
    pub fn identity(size: usize) -> Self {
        let mut out = Self::new_default([size, size]);

        for i in 0..size {
            out[[i, i]] = 1.0;
        }

        out
    }
}
