use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;
use stormath::sparse_matrix::linalg::IterativeSolverSettings;

use super::MATRIX_ROW_LENGTH;

#[derive(Debug, Clone)]
pub struct PressureSolverJacobi {
    pub matrix: SparseMatrix<MATRIX_ROW_LENGTH>,
    pub work: Vec<Float>,
    pub solver_settings: IterativeSolverSettings
}

impl PressureSolverJacobi {
    pub fn solve(&mut self, pressure: &mut [Float], rhs: &[Float]) {
        self.matrix.solve_jacobi_into(
            rhs, 
            pressure, 
            &mut self.work, 
            self.solver_settings.max_number_of_iterations,
            self.solver_settings.jacobi_weight
        );
    }
}