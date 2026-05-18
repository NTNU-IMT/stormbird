use rayon::prelude::*;

use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;
use stormath::sparse_matrix::linalg::IterativeSolverSettings;

use crate::{boundary_conditions::BoundaryConditions, grid::Grid};

use super::{
    MATRIX_ROW_LENGTH,
    PressureSolver
};

pub struct PressureSolverMultiGrid {
    pub matrices: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>>,
    pub solver_settings: IterativeSolverSettings,
    pub restriction_operators: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>>,
    pub prolongation_operators: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>>,
    pub x_at_levels: Vec<Vec<Float>>,
    pub x_at_levels_work: Vec<Vec<Float>>,
    pub rhs_at_levels: Vec<Vec<Float>>,
    pub residuals: Vec<Vec<Float>>,
    pub corrections: Vec<Vec<Float>>
}

const SMALLEST_NR_CELLS: usize = 2;

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

        let mut x_at_levels: Vec<Vec<Float>> = Vec::new();
        let mut rhs_at_levels: Vec<Vec<Float>> = Vec::new();

        let mut current_grid = grid.clone();

        while grid_can_get_coarser {
            let (matrix, _) = PressureSolver::poisson_matrix_and_rhs(
                &current_grid, 
                boundary_conditions
            );

            matrices.push(matrix);
            
            restriction_operators.push(current_grid.restriction_operator());
            prolongation_operators.push(current_grid.prolongation_operator());

            let nr_interior_cells_current = current_grid.nr_interior_cells();

            let nr_cells_total = nr_interior_cells_current[0] * 
                nr_interior_cells_current[1] *
                nr_interior_cells_current[2];

            x_at_levels.push(
                vec![0.0; nr_cells_total]
            );

            rhs_at_levels.push(
                vec![0.0; nr_cells_total]
            );

            if nr_interior_cells_current[0] % 2 != 0 ||
                nr_interior_cells_current[1] % 2 != 0 ||
                nr_interior_cells_current[2] % 2 != 0 {
                    grid_can_get_coarser = false
            } else {
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
        }

        let x_at_levels_work = x_at_levels.clone();
        let residuals = x_at_levels.clone();
        let corrections = x_at_levels.clone();

        Self {
            matrices,
            solver_settings,
            restriction_operators,
            prolongation_operators,
            x_at_levels,
            x_at_levels_work,
            rhs_at_levels,
            residuals,
            corrections
        }        
    }
    
    /// Computes the residual r = rhs - A*x at the given grid level,
    /// storing the result in self.residuals[grid_index].
    pub fn compute_residual(&mut self, grid_index: usize) {
        // Compute A*x into the residual buffer
        self.matrices[grid_index].vector_multiply_parallel(
            &self.x_at_levels[grid_index],
            &mut self.residuals[grid_index]
        );
        
        // Compute r = rhs - A*x in place (parallel)
        self.residuals[grid_index]
            .par_iter_mut()
            .zip(self.rhs_at_levels[grid_index].par_iter())
            .for_each(|(r, rhs)| {
                *r = *rhs - *r;
            });
    }
    
    pub fn perform_v_cycle(
        &mut self,
        rhs: &[Float]
    ) {
        let nr_grids = self.matrices.len();

        self.rhs_at_levels[0].copy_from_slice(rhs);

        let nr_iterations = self.solver_settings.max_number_of_iterations;

        // Smooth and restrict down to the coarsest level
        for i_g in 0..nr_grids-1 {
            if i_g > 0 {
                self.x_at_levels[i_g].fill(0.0);
            }

            self.matrices[i_g].solve_jacobi_into(
                &self.rhs_at_levels[i_g], 
                &mut self.x_at_levels[i_g], 
                &mut self.x_at_levels_work[i_g], 
                nr_iterations
            );
            
            self.compute_residual(i_g);

            self.restriction_operators[i_g].vector_multiply_parallel(
                &self.residuals[i_g], &mut self.rhs_at_levels[i_g+1]
            );
        }

        // Solve at the coarsest level
        self.x_at_levels[nr_grids-1].fill(0.0);
        
        self.matrices[nr_grids-1].solve_jacobi_into(
            &self.rhs_at_levels[nr_grids-1],
            &mut self.x_at_levels[nr_grids-1],
            &mut self.x_at_levels_work[nr_grids-1],
            nr_iterations * 2
        );
        
        // Prolongate and smooth back up
        for i_g in (0..nr_grids-1).rev() {
            self.prolongation_operators[i_g].vector_multiply_parallel(
                &self.x_at_levels[i_g + 1], &mut self.corrections[i_g]
            );

            self.x_at_levels[i_g]
                .par_iter_mut()
                .zip(self.corrections[i_g].par_iter())
                .for_each(|(x, c)| {
                    *x += *c;
                });

            self.matrices[i_g].solve_jacobi_into(
                &self.rhs_at_levels[i_g], 
                &mut self.x_at_levels[i_g], 
                &mut self.x_at_levels_work[i_g], 
                nr_iterations
            );
        }
    }

    pub fn solve(&mut self, solution: &mut [Float], rhs: &[Float]) {
        self.x_at_levels[0].copy_from_slice(solution);

        for _ in 0..self.solver_settings.min_number_of_iterations {
            self.perform_v_cycle(rhs);
        }

        solution.copy_from_slice(&self.x_at_levels[0]);
    }
}
