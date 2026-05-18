// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::type_aliases::Float;

pub mod operators;
pub mod linalg;


#[derive(Debug, Clone)]
pub struct SparseMatrix<const N: usize, > {
    pub values: Vec<[Float; N]>,
    pub diagonals: Vec<Float>,
    pub inv_diagonals: Vec<Float>,
    pub col_indices: Vec<[usize; N]>,
    pub row_length: Vec<usize>,
    pub shape: [usize; 2]
}

impl <const N: usize> SparseMatrix<N> {
    pub fn new_default(nr_rows: usize, nr_cols: usize) -> Self {
        let values = vec![[0.0; N]; nr_rows];
        let col_indices = vec![[0; N]; nr_rows];
        let row_length = vec![0; nr_rows];

        let diagonals = vec![0.0; nr_rows];
        let inv_diagonals = vec![0.0; nr_rows];
        
        Self {
            values,
            diagonals,
            inv_diagonals,
            col_indices,
            row_length,
            shape: [nr_rows, nr_cols]
        }
    }
    
    pub fn nr_rows(&self) -> usize {
        self.values.len()
    }
    
    pub fn row_entries(&self, row: usize) -> (&[Float], &[usize]) {
        let len = self.row_length[row];
        
        (&self.values[row][0..len], &self.col_indices[row][0..len])
    }

    pub fn set_diagonal_data(&mut self) {
        for i in 0..self.nr_rows() {
            self.diagonals[i] = self[[i, i]];
            
            if self.diagonals[i] != 0.0 {
                self.inv_diagonals[i] = 1.0 / self.diagonals[i];
            }
        }
    }
}
