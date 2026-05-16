use super::*;

use std::ops::{Index, IndexMut};

impl<const N: usize> Index<[usize; 2]> for SparseMatrix<N> {
    type Output = Float;

    fn index(&self, indices: [usize; 2]) -> &Self::Output {
        let row_index = indices[0];
        let col_index = indices[1];

        if row_index >= self.shape[0] {
            panic!("Row index out of bounds");
        }

        if col_index >= self.shape[1] {
            panic!("Column index out of bounds");
        }
        
        for i_col_local in 0..self.row_length[row_index] {
            if self.col_indices[row_index][i_col_local] == col_index {
                return &self.values[row_index][i_col_local];
            }
        }
        
        &0.0
    }
}

impl<const N: usize> IndexMut<[usize; 2]> for SparseMatrix<N> {
    fn index_mut(&mut self, indices: [usize; 2]) -> &mut Self::Output {
        let row_index = indices[0];
        let col_index = indices[1];

        if row_index >= self.shape[0] {
            panic!("Row index out of bounds");
        }

        if col_index >= self.shape[1] {
            panic!("Column index out of bounds");
        }
        
        for i_col_local in 0..self.row_length[row_index] {
            if self.col_indices[row_index][i_col_local] == col_index {
                return &mut self.values[row_index][i_col_local];
            }
        }
        
        if self.row_length[row_index] < N {
            let new_col_index = self.row_length[row_index];
            
            self.row_length[row_index] += 1;
            self.col_indices[row_index][new_col_index] = col_index;
            
            return &mut self.values[row_index][new_col_index]
        }
        
        panic!("Column index is not in the row, and the row is already full");
    }
}

impl <const N: usize> SparseMatrix<N> {
    /// Performs a matrix multiplication with the supplied vector x
    pub fn vector_multiply(&self, x: &[Float]) -> Vec<Float> {
        assert_eq!(
            self.shape[1], 
            x.len(), 
            "Matrix and vector shapes do not match for multiplication"
        );

        let mut result = vec![0.0; self.shape[0]];

        for i in 0..self.shape[0] {
            let mut sum = 0.0;

            for j in 0..self.row_length[i] {
                let col_index = self.col_indices[i][j];

                sum = sum + self.values[i][j] * x[col_index];
            }

            result[i] = sum
        }

        result
    }
}