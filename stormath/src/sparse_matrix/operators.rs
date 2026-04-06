use super::*;

use std::ops::{Index, IndexMut};

impl<const N: usize> Index<[usize; 2]> for SparseMatrix<N> {
    type Output = Float;

    fn index(&self, indices: [usize; 2]) -> &Self::Output {
        let row_index = indices[0];
        let col_index = indices[1];
        
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