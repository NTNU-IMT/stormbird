use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;
use stormath::matrix::linalg::IterativeSolverSettings;

use super::MATRIX_ROW_LENGTH;

#[derive(Debug, Clone)]
pub struct PressureSolverJacobi {
    pub matrix: SparseMatrix<MATRIX_ROW_LENGTH>,
    pub solver_settings: IterativeSolverSettings
}

impl PressureSolverJacobi {
    pub fn solve(&self, initial_guess: &[Float], rhs: &[Float])-> Vec<Float> {        
        let pressure_interior = self.matrix.solve_jacobi(
            rhs, &initial_guess, &self.solver_settings
        ).unwrap();
        
        pressure_interior
    }
}