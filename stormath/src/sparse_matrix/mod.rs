// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! A structure for a simple sparse matrix - i.e., a matrix where most of the entries in one row is
//! zero - along with functionality for this type of matrices.

use crate::type_aliases::Float;

pub mod operators;
pub mod linalg;


#[derive(Debug, Clone)]
/// This sparse matrix structure is defined to have a fixed size maximum number of non-zero column 
/// entries, using const generics. However, the actual number of column entries can vary between 
/// zero and the maximum value.  
pub struct SparseMatrix<const N: usize, > {
    /// Values stored in a vector with arrays of fixed length, N. The length of the vector equals 
    /// the number of rows for the matrix, while N specifies the max number of values to store per
    /// row.
    pub values: Vec<[Float; N]>,
    /// The column indices for each entry in the values vector. 
    pub col_indices: Vec<[usize; N]>,
    /// The length of each row, that should have numbers between 0 and N
    pub row_length: Vec<usize>,
    /// The general shape of the matrix [number of rows, number of columns]
    pub shape: [usize; 2]
}

impl <const N: usize> SparseMatrix<N> {
    /// Creates a new matrix with the given shape where all values are set to zero
    pub fn new_default(nr_rows: usize, nr_cols: usize) -> Self {
        let values = vec![[0.0; N]; nr_rows];
        let col_indices = vec![[0; N]; nr_rows];
        let row_length = vec![0; nr_rows];

        Self {
            values,
            col_indices,
            row_length,
            shape: [nr_rows, nr_cols]
        }
    }

    /// Returns the number of rows
    pub fn nr_rows(&self) -> usize {
        self.values.len()
    }

    /// Returns all the entries for a row, along with the column indices that these values belong to
    pub fn row_entries(&self, row: usize) -> (&[Float], &[usize]) {
        let len = self.row_length[row];
        
        (&self.values[row][0..len], &self.col_indices[row][0..len])
    }
}
