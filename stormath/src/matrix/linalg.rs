use super::*;

impl Matrix<f64> {
    /// Solves the equation system Ax = b using Gaussian elimination.
    /// 
    /// Source: <https://en.wikipedia.org/wiki/Gaussian_elimination>
    pub fn solve_gaussian_elimination(&self, rhs: &[f64]) -> Vec<f64> {
        let n = self.nr_rows();
        let m = self.nr_cols();
        
        // Check that the matrix is square
        assert_eq!(n, m, "Matrix must be square for Gaussian elimination");
        assert_eq!(rhs.len(), n, "Right-hand side vector length must match matrix size");
        
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
                panic!("Matrix is singular or nearly singular");
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
        
        x
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

        let x_solved = a.solve_gaussian_elimination(&b);

        let x_numpy = vec![0.6, -0.4,  2.0]; // Manually extracted from NumPy

        dbg!(&x_solved);

        for i in 0..x_solved.len() {
            assert!((x_solved[i] - x_numpy[i]).abs() < 1e-12, "Mismatch at index {}: {} != {}", i, x_solved[i], x_numpy[i]);
        }
    }
}