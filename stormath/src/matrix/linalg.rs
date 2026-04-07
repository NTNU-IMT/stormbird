// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation block for the matric data type for linear algebra operations. This includes the
//! following:
//!
//! - [`solve_gaussian_elimination`](Matrix::solve_gaussian_elimination)
//! - [`solve_gauss_seidel`](Matrix::solve_gauss_seidel)

use super::*;

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
    pub relative_residual_limit: Float
}

impl IterativeSolverSettings {
    fn default_min_number_of_iterations() -> usize {1}
    fn default_max_number_of_iterations() -> usize {1000}
    fn default_relative_residual_limit() -> Float {0.00001}
}

impl Default for IterativeSolverSettings {
    fn default() -> Self {
        Self {
            min_number_of_iterations: Self::default_min_number_of_iterations(),
            max_number_of_iterations: Self::default_max_number_of_iterations(),
            relative_residual_limit: Self::default_relative_residual_limit()
        }
    }
}

impl Matrix<Float>{
    pub fn check_dimensions_for_solvability(&self, rhs: &[Float]) {
        let n = self.nr_rows();
        let m = self.nr_cols();

        // Check that the matrix is square
        assert_eq!(n, m, "Matrix must be square to be solvable");
        assert_eq!(rhs.len(), n, "Right-hand side vector length must match matrix size");

    }
    
    pub fn linear_residual(&self, rhs: &[Float], x: &[Float]) -> Vec<Float> {    
        let ax = self.vector_multiply(x);
        
        ax.iter().zip(rhs.iter()).map(|(a, b)| b - a).collect()
    }
    
    pub fn linear_residual_absolute_sum(&self, rhs: &[Float], x: &[Float]) -> Float {
        let r = self.linear_residual(rhs, x);
        
        r.iter().map(|r| r.abs()).sum::<Float>() / (r.len() as Float)
    }

    /// Solves the equation system Ax = b using Gaussian elimination.
    ///
    /// Source: <https://en.wikipedia.org/wiki/Gaussian_elimination>
    pub fn solve_gaussian_elimination(&self, rhs: &[Float]) -> Result<Vec<Float>, Error> {
        self.check_dimensions_for_solvability(rhs);

        let n = self.nr_rows();

        // Create augmented matrix [A|b]
        let mut aug: Matrix<Float> = Matrix::new_default([n, n + 1]);

        // Copy matrix A and vector b into augmented matrix
        for i in 0..n {
            for j in 0..n {
                aug[[i, j]] = self[[i, j]];
            }
            aug[[i, n]] = rhs[i];
        }

        // Forward elimination with partial pivoting
        for k in 0..n {
            // Find the pivot (largest absolute value in column k, from row k onwards)
            let mut max_row = k;
            for i in (k + 1)..n {
                if aug[[i, k]].abs() > aug[[max_row, k]].abs() {
                    max_row = i;
                }
            }

            // Swap rows if needed
            if max_row != k {
                for j in 0..=n {
                    let temp = aug[[k, j]];
                    aug[[k, j]] = aug[[max_row, j]];
                    aug[[max_row, j]] = temp;
                }
            }

            // Check for singular matrix (pivot is effectively zero)
            if aug[[k, k]].abs() < 1e-12 {
                return Err(
                    Error::NoSolution(format!("Matrix is singular or nearly singular at row {}", k))
                );
            }

            // Eliminate column k in rows below k
            for i in (k + 1)..n {
                let factor = aug[[i, k]] / aug[[k, k]];
                for j in k..=n {
                    aug[[i, j]] -= factor * aug[[k, j]];
                }
            }
        }

        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = 0.0;
            for j in (i + 1)..n {
                sum += aug[[i, j]] * x[j];
            }

            x[i] = (aug[[i, n]] - sum) / aug[[i, i]];
        }

        Ok(x)
    }
    
    /// Solves the equation system Ax = b using Gauss-Seidel method.
    ///
    /// Source: <https://en.wikipedia.org/wiki/Gauss%E2%80%93Seidel_method>
    pub fn solve_gauss_seidel(
        &self, 
        rhs: &[Float], 
        initial_guess: &[Float], 
        settings: &IterativeSolverSettings
    ) -> Result<Vec<Float>, Error> {
        self.check_dimensions_for_solvability(rhs);

        let n = self.nr_rows();

        let mut estimated_solution = initial_guess.to_vec();
        
        let initial_residual = self.linear_residual_absolute_sum(rhs, &estimated_solution);
        
        for i in 0..n {
            if self[[i, i]].abs() < 1e-12 {
                return Err(
                    Error::NoSolution(format!("Matrix is singular or nearly singular at row {}", i))
                );
            }
        }
        
        let diag_inv: Vec<Float> = (0..n).map(|i| 1.0 / self[[i, i]]).collect();

        for iteration in 0..settings.max_number_of_iterations {
            for i in 0..n {
                let row = self.row_slice(i);
                
                let sigma =
                    row[..i].iter().zip(&estimated_solution[..i])
                        .map(|(a, x)| a * x).sum::<Float>()
                  + row[i+1..].iter().zip(&estimated_solution[i+1..])
                        .map(|(a, x)| a * x).sum::<Float>();
                
                estimated_solution[i] = (rhs[i] - sigma) * diag_inv[i];
            }
            
            
            if iteration > settings.min_number_of_iterations {
                let new_residual = self.linear_residual_absolute_sum(rhs, &estimated_solution);
                
                if new_residual / initial_residual < settings.relative_residual_limit {
                    break
                }
            }
            
        }

        Ok(estimated_solution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_solver() {
        let allowable_error = 1e-4;

        let a = Matrix{
            data: vec![3.0, 2.0, 0.0,
                       2.0, 3.0, 1.0,
                      -1.0, 1.0, 2.0],
            shape: [3, 3],
        };

        let b = vec![1.0, 2.0, 3.0];

        let x_solved_elimination = a.solve_gaussian_elimination(&b).unwrap();
        
        let initial_guess = vec![0.0; 3];
        
        let x_solved_iterative = a.solve_gauss_seidel(
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
