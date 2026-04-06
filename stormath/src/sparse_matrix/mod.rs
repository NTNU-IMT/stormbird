// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::type_aliases::Float;

pub mod operators;
pub mod linalg;


#[derive(Debug, Clone)]
pub struct SparseMatrix<const N: usize, > {
    pub values: Vec<[Float; N]>,
    pub col_indices: Vec<[usize; N]>,
    pub row_length: Vec<usize>
}

impl <const N: usize> SparseMatrix<N> {
    pub fn new_default(nr_rows: usize) -> Self {
        let values = vec![[0.0; N]; nr_rows];
        let col_indices = vec![[0; N]; nr_rows];
        let row_length = vec![0; nr_rows];
        
        Self {
            values,
            col_indices,
            row_length
        }
    }
    
    pub fn nr_rows(&self) -> usize {
        self.values.len()
    }
    
    pub fn row_entries(&self, row: usize) -> (&[Float], &[usize]) {
        let len = self.row_length[row];
        
        (&self.values[row][0..len], &self.col_indices[row][0..len])
    }
}