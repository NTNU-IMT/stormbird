use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;
use stormath::matrix::linalg::IterativeSolverSettings;

use crate::{boundary_conditions::BoundaryConditions, grid::Grid};

use super::{
    MATRIX_ROW_LENGTH,
    PressureSolver
};

pub struct PressureSolverMultiGrid {
    pub matrices: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>>,
    pub solver_settings: IterativeSolverSettings,
    pub restriction_operators: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>>,
    pub prolongation_operators: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>>
}

const SMALLEST_NR_CELLS: usize = 8;

impl PressureSolverMultiGrid {
    pub fn new(
        grid: &Grid, 
        boundary_conditions: &BoundaryConditions, 
        solver_settings: IterativeSolverSettings
    ) -> Self {
        let mut grid_can_get_coarser = true;

        let mut matrices: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>> = Vec::new();
        let mut restriction_operators: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>> = Vec::new();
        let mut prolongation_operators: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>> = Vec::new();

        let mut current_grid = grid.clone();

        while grid_can_get_coarser {
            let (matrix, _) = PressureSolver::poisson_matrix_and_rhs(
                &current_grid, 
                boundary_conditions
            );

            matrices.push(matrix);
            
            restriction_operators.push(current_grid.restriction_operator());
            prolongation_operators.push(current_grid.prolongation_operator());

            let coarser_grid = current_grid.coarsened();
            let nr_interior_cells = coarser_grid.nr_interior_cells();

            if nr_interior_cells[0] > SMALLEST_NR_CELLS && 
                nr_interior_cells[1] > SMALLEST_NR_CELLS && 
                nr_interior_cells[2] > SMALLEST_NR_CELLS {
                current_grid = coarser_grid
            } else {
                grid_can_get_coarser = false
            }
        }

        Self {
            matrices,
            solver_settings,
            restriction_operators,
            prolongation_operators
        }        
    }
    
    pub fn compute_residual(
        &self, 
        grid_index: usize,
        x_current: &[Float],
        rhs_current: &[Float]
    ) -> Vec<Float> {
        let matrix_product = self.matrices[grid_index].vector_multiply(x_current);
        
        rhs_current.iter().zip(matrix_product.iter()).map(|(r, ax)| r - ax).collect()
    }
    
    pub fn perform_v_cycle(
        &self, 
        initial_guess: &[Float], 
        rhs: &[Float]
    ) -> Vec<Float> {
        let nr_grids = self.matrices.len();

        let mut x_at_levels: Vec<Vec<Float>> = Vec::with_capacity(nr_grids);
        let mut rhs_at_levels: Vec<Vec<Float>> = Vec::with_capacity(nr_grids);

        rhs_at_levels.push(rhs.to_vec());

        // Smooth and restrict down to the coarsest level
        for i_g in 0..nr_grids {
            let x0 = if i_g == 0 {
                initial_guess.to_vec()
            } else {
                vec![0.0; rhs_at_levels[i_g].len()]
            };
            
            x_at_levels.push(
                self.matrices[i_g].solve_jacobi(
                    &rhs_at_levels[i_g], &x0, &self.solver_settings
                ).unwrap()
            );
            
            let residual = self.compute_residual(i_g, &x_at_levels[i_g], &rhs_at_levels[i_g]);

            rhs_at_levels.push(
                self.restriction_operators[i_g].vector_multiply(&residual)
            );
        }
        
        // Prolongate and smooth back up
        for i_g in (0..nr_grids-1).rev() {
            let correction = self.prolongation_operators[i_g].vector_multiply(
                &x_at_levels[i_g + 1]
            );

            // Apply correction to the current solution
            let corrected: Vec<Float> = x_at_levels[i_g].iter()
                .zip(correction.iter())
                .map(|(x, c)| x + c)
                .collect();
            
            x_at_levels[i_g] = self.matrices[i_g].solve_jacobi(
                &rhs_at_levels[i_g], &corrected, &self.solver_settings
            ).unwrap()
        }
        
        x_at_levels[0].clone()
    }

    pub fn solve(&self, initial_guess: &[Float], rhs: &[Float]) -> Vec<Float> {
        let mut x_current = initial_guess.to_vec();

        for _ in 0..self.solver_settings.min_number_of_iterations {
            x_current = self.perform_v_cycle(&x_current, rhs);
        }

        x_current
    }
}
