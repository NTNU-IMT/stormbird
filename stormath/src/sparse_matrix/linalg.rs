use super::*;

use rayon::prelude::*;

use crate::matrix::linalg::IterativeSolverSettings;
use crate::error::Error;


impl<const N: usize> SparseMatrix<N> {
    /// Solves the equation system Ax = b using the Jacobi method.
    ///
    /// Source: <https://en.wikipedia.org/wiki/Jacobi_method>
    pub fn solve_jacobi(
        &self, 
        rhs: &[Float], 
        initial_guess: &[Float], 
        settings: &IterativeSolverSettings
    ) -> Result<Vec<Float>, Error> {

        let n = self.nr_rows();

        let mut current_solution = initial_guess.to_vec();
        let mut new_solution = initial_guess.to_vec();
        
        let diag: Vec<Float> = (0..n).map(|i| self[[i, i]]).collect();
        let diag_inv: Vec<Float> = (0..n).map(|i| 1.0 / self[[i, i]]).collect();

        for _iteration in 0..settings.max_number_of_iterations {
            new_solution
                .par_iter_mut()
                .enumerate()
                .for_each(|(i_row, x_i_row)| {
                    let (row_values, col_indices) = self.row_entries(i_row);
                    
                    let mut full_dot = 0.0;
                    
                    for i_col_local in 0..row_values.len() {
                        let i_col = col_indices[i_col_local];
                        
                        full_dot += current_solution[i_col] * row_values[i_col_local];
                    }

                    let sigma = full_dot - diag[i_row] * current_solution[i_row];
                    *x_i_row = (rhs[i_row] - sigma) * diag_inv[i_row];
                });

            std::mem::swap(&mut current_solution, &mut new_solution);
        }

        Ok(current_solution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::matrix::Matrix;

    #[test]
    fn test_sparse_matrix_solver() {
        let allowable_error = 1e-4;

        let a = Matrix{
            data: vec![3.0, 2.0, 0.0,
                       2.0, 3.0, 1.0,
                      -1.0, 1.0, 2.0],
            shape: [3, 3],
        };
        
        let mut a_sparse: SparseMatrix<3> = SparseMatrix::new_default(3);
        
        for i in 0..3 {
            for j in 0..3 {
                a_sparse[[i, j]] = a[[i, j]];
            }
        }
        
        dbg!(&a_sparse);

        let b = vec![1.0, 2.0, 3.0];

        let x_solved_elimination = a.solve_gaussian_elimination(&b).unwrap();
        
        let initial_guess = vec![0.0; 3];
        
        let x_solved_iterative = a_sparse.solve_jacobi(
            &b, 
            &initial_guess, 
            &IterativeSolverSettings::default()
        ).unwrap();

        let x_numpy = vec![0.6, -0.4,  2.0]; // Manually extracted from NumPy

        dbg!(&x_solved_elimination);
        dbg!(&x_solved_iterative);

        for i in 0..x_solved_elimination.len() {
            assert!(
                (x_solved_elimination[i] - x_numpy[i]).abs() < allowable_error,
                "Mismatch at index {}: {} != {}",
                i, x_solved_elimination[i], x_numpy[i]
            );

            assert!(
                (x_solved_iterative[i] - x_numpy[i]).abs() < allowable_error,
                "Mismatch at index {}: {} != {}",
                i, x_solved_iterative[i], x_numpy[i]
            );
        }
    }
}