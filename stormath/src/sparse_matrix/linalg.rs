use super::*;

use rayon::prelude::*;

use crate::matrix::Matrix;
use crate::error::Error;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IterativeSolverSettings {
    #[serde(default="IterativeSolverSettings::default_min_number_of_iterations")]
    pub min_number_of_iterations: usize,
    #[serde(default="IterativeSolverSettings::default_max_number_of_iterations")]
    pub max_number_of_iterations: usize,
    #[serde(default="IterativeSolverSettings::default_relative_residual_limit")]
    pub relative_residual_limit: Float,
    /// Relaxation weight for weighted Jacobi iteration (omega).
    /// - omega = 1.0: Standard Jacobi
    /// - omega = 2/3 ≈ 0.667: Optimal for Laplacian on structured grids
    /// - omega < 1.0: Under-relaxation (more stable, slower convergence)
    /// - omega > 1.0: Over-relaxation (can be unstable)
    #[serde(default="IterativeSolverSettings::default_jacobi_weight")]
    pub jacobi_weight: Float
}

impl IterativeSolverSettings {
    fn default_min_number_of_iterations() -> usize {1}
    fn default_max_number_of_iterations() -> usize {1000}
    fn default_relative_residual_limit() -> Float {0.00001}
    /// Default weight of 2/3 is optimal for Laplacian on structured grids
    fn default_jacobi_weight() -> Float { 2.0 / 3.0 }
}

impl Default for IterativeSolverSettings {
    fn default() -> Self {
        Self {
            min_number_of_iterations: Self::default_min_number_of_iterations(),
            max_number_of_iterations: Self::default_max_number_of_iterations(),
            relative_residual_limit: Self::default_relative_residual_limit(),
            jacobi_weight: Self::default_jacobi_weight()
        }
    }
}

impl<const N: usize> SparseMatrix<N> {
    /// Converts the sparse matrix to a dense Matrix.
    /// 
    /// This is useful when an exact solver is needed, as Gaussian elimination
    /// causes fill-in that cannot be efficiently handled by the fixed-row-length
    /// sparse format.
    pub fn to_dense(&self) -> Matrix<Float> {
        let mut dense = Matrix::new_default(self.shape);
        
        for i_row in 0..self.shape[0] {
            let (row_values, col_indices) = self.row_entries(i_row);
            
            for (i_local, &i_col) in col_indices.iter().enumerate() {
                dense[[i_row, i_col]] = row_values[i_local];
            }
        }
        
        dense
    }
    
    /// Solves the equation system Ax = b exactly using Gaussian elimination.
    /// 
    /// This method converts the sparse matrix to a dense format and uses
    /// Gaussian elimination with partial pivoting. It is best suited for
    /// small systems (e.g., coarsest grid in multigrid) where the overhead
    /// of dense storage is acceptable.
    /// 
    /// For large sparse systems, use iterative methods like `solve_jacobi` instead.
    pub fn solve_exact(&self, rhs: &[Float]) -> Result<Vec<Float>, Error> {
        let dense = self.to_dense();
        dense.solve_gaussian_elimination(rhs)
    }
    
    /// Solves the equation system Ax = b using the Jacobi method.
    ///
    /// Source: <https://en.wikipedia.org/wiki/Jacobi_method>
    pub fn solve_jacobi(
        &self, 
        rhs: &[Float], 
        initial_guess: &[Float], 
        nr_iterations: usize
    ) -> Result<Vec<Float>, Error> {

        let n = self.nr_rows();

        let mut current_solution = initial_guess.to_vec();
        let mut new_solution = initial_guess.to_vec();
        
        let diag: Vec<Float> = (0..n).map(|i| self[[i, i]]).collect();
        let diag_inv: Vec<Float> = (0..n).map(|i| 1.0 / self[[i, i]]).collect();

        for _iteration in 0..nr_iterations {
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
    
    /// Solves the equation system Ax = b using the weighted Jacobi method, writing the
    /// solution into the provided buffer.
    /// 
    /// The weighted Jacobi update is:
    ///   x_new = (1 - ω) * x_old + ω * D⁻¹(b - (L + U) * x_old)
    /// 
    /// where ω is the relaxation weight:
    /// - ω = 1.0: Standard Jacobi
    /// - ω = 2/3: Optimal for Laplacian on structured grids (default)
    /// 
    /// # Arguments
    /// * `rhs` - The right-hand side vector b
    /// * `solution` - On input: the initial guess. On output: the computed solution.
    /// * `work` - A scratch buffer of the same size as `solution` for intermediate computations.
    /// * `nr_iterations` - Number of iterations to run
    /// * `omega` - Relaxation weight (use 1.0 for standard Jacobi, 2/3 for optimal Laplacian smoothing)
    ///
    /// Source: <https://en.wikipedia.org/wiki/Jacobi_method>
    pub fn solve_jacobi_into(
        &self, 
        rhs: &[Float], 
        solution: &mut [Float],
        work: &mut [Float],
        nr_iterations: usize,
        omega: Float
    ) {
        let one_minus_omega = 1.0 - omega;
        
        for iteration in 0..nr_iterations {
            // Determine which buffer is current and which is new based on iteration parity
            let (current, new) = if iteration % 2 == 0 {
                (solution as &[Float], work as &mut [Float])
            } else {
                (work as &[Float], solution as &mut [Float])
            };
            
            new
                .par_iter_mut()
                .enumerate()
                .for_each(|(i_row, x_i_row)| {
                    let (row_values, col_indices) = self.row_entries(i_row);
                    
                    let mut full_dot = 0.0;
                    for i_col_local in 0..row_values.len() {
                        let i_col = col_indices[i_col_local];
                        full_dot += current[i_col] * row_values[i_col_local];
                    }

                    let sigma = full_dot - self.diagonals[i_row] * current[i_row];
                    let jacobi_update = (rhs[i_row] - sigma) * self.inv_diagonals[i_row];
                    
                    // Weighted Jacobi: x_new = (1 - ω) * x_old + ω * jacobi_update
                    *x_i_row = one_minus_omega * current[i_row] + omega * jacobi_update;
                });
        }
        
        // If odd number of iterations, result is in work buffer; copy to solution
        if nr_iterations % 2 == 1 {
            solution.copy_from_slice(work);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::matrix::Matrix;

    #[test]
    fn test_sparse_matrix_to_dense() {
        let a = Matrix {
            data: vec![3.0, 2.0, 0.0,
                       2.0, 3.0, 1.0,
                      -1.0, 1.0, 2.0],
            shape: [3, 3],
        };
        
        let mut a_sparse: SparseMatrix<3> = SparseMatrix::new_default(3, 3);
        
        for i in 0..3 {
            for j in 0..3 {
                if a[[i, j]] != 0.0 {
                    a_sparse[[i, j]] = a[[i, j]];
                }
            }
        }
        
        let a_dense = a_sparse.to_dense();
        
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (a[[i, j]] - a_dense[[i, j]]).abs() < 1e-10,
                    "Mismatch at [{}, {}]: {} != {}",
                    i, j, a[[i, j]], a_dense[[i, j]]
                );
            }
        }
    }

    #[test]
    fn test_sparse_matrix_solve_exact() {
        let allowable_error = 1e-10;

        let a = Matrix {
            data: vec![3.0, 2.0, 0.0,
                       2.0, 3.0, 1.0,
                      -1.0, 1.0, 2.0],
            shape: [3, 3],
        };
        
        let mut a_sparse: SparseMatrix<3> = SparseMatrix::new_default(3, 3);
        
        for i in 0..3 {
            for j in 0..3 {
                if a[[i, j]] != 0.0 {
                    a_sparse[[i, j]] = a[[i, j]];
                }
            }
        }

        let b = vec![1.0, 2.0, 3.0];

        let x_solved = a_sparse.solve_exact(&b).unwrap();
        let x_expected = vec![0.6, -0.4, 2.0];

        for i in 0..x_solved.len() {
            assert!(
                (x_solved[i] - x_expected[i]).abs() < allowable_error,
                "Mismatch at index {}: {} != {}",
                i, x_solved[i], x_expected[i]
            );
        }
    }

    #[test]
    fn test_sparse_matrix_solver() {
        let allowable_error = 1e-4;

        let a = Matrix{
            data: vec![3.0, 2.0, 0.0,
                       2.0, 3.0, 1.0,
                      -1.0, 1.0, 2.0],
            shape: [3, 3],
        };
        
        let mut a_sparse: SparseMatrix<3> = SparseMatrix::new_default(3, 3);
        
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
            1000
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

    #[test]
    fn test_sparse_matrix_solve_jacobi_into() {
        let allowable_error = 1e-4;

        let a = Matrix {
            data: vec![3.0, 2.0, 0.0,
                       2.0, 3.0, 1.0,
                      -1.0, 1.0, 2.0],
            shape: [3, 3],
        };
        
        let mut a_sparse: SparseMatrix<3> = SparseMatrix::new_default(3, 3);
        
        for i in 0..3 {
            for j in 0..3 {
                a_sparse[[i, j]] = a[[i, j]];
            }
        }

        let b = vec![1.0, 2.0, 3.0];
        let x_expected = vec![0.6, -0.4, 2.0];

        // Test with even number of iterations (omega=1.0 for standard Jacobi)
        let mut solution_even = vec![0.0; 3];
        let mut work_even = vec![0.0; 3];
        
        a_sparse.solve_jacobi_into(&b, &mut solution_even, &mut work_even, 1000, 1.0);

        for i in 0..solution_even.len() {
            assert!(
                (solution_even[i] - x_expected[i]).abs() < allowable_error,
                "Even iterations: Mismatch at index {}: {} != {}",
                i, solution_even[i], x_expected[i]
            );
        }

        // Test with odd number of iterations (omega=1.0 for standard Jacobi)
        let mut solution_odd = vec![0.0; 3];
        let mut work_odd = vec![0.0; 3];
        
        a_sparse.solve_jacobi_into(&b, &mut solution_odd, &mut work_odd, 1001, 1.0);

        for i in 0..solution_odd.len() {
            assert!(
                (solution_odd[i] - x_expected[i]).abs() < allowable_error,
                "Odd iterations: Mismatch at index {}: {} != {}",
                i, solution_odd[i], x_expected[i]
            );
        }

        // Verify solve_jacobi and solve_jacobi_into produce the same result
        let x_from_solve_jacobi = a_sparse.solve_jacobi(&b, &vec![0.0; 3], 1000).unwrap();

        for i in 0..solution_even.len() {
            assert!(
                (solution_even[i] - x_from_solve_jacobi[i]).abs() < 1e-10,
                "solve_jacobi vs solve_jacobi_into mismatch at index {}: {} != {}",
                i, solution_even[i], x_from_solve_jacobi[i]
            );
        }
    }
}
