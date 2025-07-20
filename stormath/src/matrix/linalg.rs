use super::*;

use crate::error::Error;

impl Matrix<f64> {
    pub fn check_dimensions_for_solvability(&self, rhs: &[f64]) {
        let n = self.nr_rows();
        let m = self.nr_cols();
        
        // Check that the matrix is square
        assert_eq!(n, m, "Matrix must be square to be solvable");
        assert_eq!(rhs.len(), n, "Right-hand side vector length must match matrix size");

    }

    /// Solves the equation system Ax = b using Gaussian elimination.
    /// 
    /// Source: <https://en.wikipedia.org/wiki/Gaussian_elimination>
    pub fn solve_gaussian_elimination(&self, rhs: &[f64]) -> Result<Vec<f64>, Error> {
        self.check_dimensions_for_solvability(rhs);

        let n = self.nr_rows();
        
        // Create augmented matrix [A|b]
        let mut aug = Matrix::new_default([n, n + 1]);
        
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
    pub fn solve_gauss_seidel(&self, rhs: &[f64], nr_iterations: usize) -> Result<Vec<f64>, Error> {
        self.check_dimensions_for_solvability(rhs);

        let n = self.nr_rows();

        let mut estimated_solution = vec![0.0; n];

        for _ in 0..nr_iterations {
            for i in 0..n {
                if self[[i, i]].abs() < 1e-12 {
                    return Err(
                        Error::NoSolution(format!("Matrix is singular or nearly singular at row {}", i))
                    );
                }

                let mut sigma = 0.0;

                for j in 0..n {
                    if i != j {
                        sigma += self[[i, j]] * estimated_solution[j];
                    }
                }

                estimated_solution[i] = (rhs[i] - sigma) / self[[i, i]]
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

        let a = Matrix{
            data: vec![3.0, 2.0, 0.0, 
                       2.0, 3.0, 1.0, 
                      -1.0, 1.0, 2.0],
            shape: [3, 3],
        };

        let b = vec![1.0, 2.0, 3.0];

        let x_solved_elimination = a.solve_gaussian_elimination(&b).unwrap();
        let x_solved_iterative = a.solve_gauss_seidel(&b, 100).unwrap();

        let x_numpy = vec![0.6, -0.4,  2.0]; // Manually extracted from NumPy

        dbg!(&x_solved_elimination);
        dbg!(&x_solved_iterative);

        for i in 0..x_solved_elimination.len() {
            assert!(
                (x_solved_elimination[i] - x_numpy[i]).abs() < 1e-12, 
                "Mismatch at index {}: {} != {}", 
                i, x_solved_elimination[i], x_numpy[i]
            );

            assert!(
                (x_solved_iterative[i] - x_numpy[i]).abs() < 1e-12, 
                "Mismatch at index {}: {} != {}", 
                i, x_solved_iterative[i], x_numpy[i]
            );
        }
    }
}