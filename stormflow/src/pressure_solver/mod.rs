pub mod builder;
pub mod jacobi;
pub mod multigrid;

use stormath::type_aliases::Float;
use stormath::sparse_matrix::SparseMatrix;

use crate::grid::Grid;
use crate::boundary_conditions::{BoundaryCondition, BoundaryConditions};

use jacobi::PressureSolverJacobi;
use multigrid::PressureSolverMultiGrid;

const MATRIX_ROW_LENGTH: usize = 9;

pub enum PressureSolver {
    Jacobi(PressureSolverJacobi),
    Multigrid(PressureSolverMultiGrid)
}

impl PressureSolver {
    pub fn solve(&self, initial_guess: &[Float], rhs: &[Float])-> Vec<Float> {
        let x = match self {
            Self::Jacobi(solver) => solver.solve(initial_guess, rhs),
            Self::Multigrid(solver) => solver.solve(initial_guess, rhs)
        };

        let matrix_product = match self {
            Self::Jacobi(solver) => solver.matrix.vector_multiply(&x),
            Self::Multigrid(solver) => solver.matrices[0].vector_multiply(&x)
        };

        let residual_sum = matrix_product
            .iter()
            .zip(rhs.iter())
            .map(|(ax, r)| (r - ax).abs())
            .sum::<Float>() / rhs.len() as Float;

        println!("Residual sum: {}", residual_sum);

        x
    }
    
    /// Sets up the equation system for the a Poisson matrix 
    pub fn poisson_matrix_and_rhs(
        grid: &Grid, 
        boundary_conditions: &BoundaryConditions
    ) -> (SparseMatrix<MATRIX_ROW_LENGTH>, Vec<Float>) {
        let [dx, dy, dz] = grid.cell_length.0;
           
        let [nx, ny, nz] = grid.nr_interior_cells();
        
        let nr_interior_cells = nx * ny * nz;
        
        let mut matrix: SparseMatrix<MATRIX_ROW_LENGTH> = SparseMatrix::new_default(
            nr_interior_cells, 
            nr_interior_cells
        );
        let mut rhs: Vec<Float> = vec![0.0; nr_interior_cells];
        
        for i_x in 0..nx {
            for i_y in 0..ny {
                for i_z in 0..nz {
                    let i_l = grid.local_flat_indices_on_interior_grid([i_x, i_y, i_z]);
                    
                    if i_x == 0 {
                        match boundary_conditions.pressure[0][0] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{i-1} = p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{-p_i + p_{i+1}}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.pos[0]]] += 1.0 / dx.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{i-1} + p_i) = value
                                // p_{i-1} = 2 * value - p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{2 * value - p_i - 2 p_i + p_{i+1}}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.pos[0]]] += 1.0 / dx.powi(2);
                                
                                rhs[i_l.current] += -2.0 * value / dx.powi(2);
                            }
                        }
                    } else if i_x == nx - 1 {
                        match boundary_conditions.pressure[0][1] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{i+1} = p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{-p_i + p_{i-1}}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.neg[0]]] += 1.0 / dx.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{i+1} + p_i) = value
                                // p_{i+1} = 2 * value - p_i
                                // \frac{p_{i-1} - 2 p_i + p_{i+1}}{dx^2} = \frac{p_{i-1} - 2 p_i + 2 * value - p_i}{dx^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dx.powi(2);
                                matrix[[i_l.current, i_l.neg[0]]] += 1.0 / dx.powi(2);
                                
                                rhs[i_l.current] += -2.0 * value / dx.powi(2);
                            }
                        }
                    } else {
                        matrix[[i_l.current, i_l.neg[0]]] += 1.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dx.powi(2);
                        matrix[[i_l.current, i_l.pos[0]]] += 1.0 / dx.powi(2);
                    }
    
                    // Y direction
                    if i_y == 0 {
                        match boundary_conditions.pressure[1][0] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{j-1} = p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{-p_j + p_{j+1}}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.pos[1]]] += 1.0 / dy.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{j-1} + p_j) = value
                                // p_{j-1} = 2 * value - p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{2 * value - p_j - 2 p_j + p_{j+1}}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.pos[1]]] += 1.0 / dy.powi(2);
    
                                rhs[i_l.current] += -2.0 * value / dy.powi(2);
                            }
                        }
                    } else if i_y == ny - 1 {
                        match boundary_conditions.pressure[1][1] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{j+1} = p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{-p_j + p_{j-1}}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.neg[1]]] += 1.0 / dy.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{j+1} + p_j) = value
                                // p_{j+1} = 2 * value - p_j
                                // \frac{p_{j-1} - 2 p_j + p_{j+1}}{dy^2} = \frac{p_{j-1} - 2 p_j + 2 * value - p_j}{dy^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dy.powi(2);
                                matrix[[i_l.current, i_l.neg[1]]] += 1.0 / dy.powi(2);
    
                                rhs[i_l.current] += -2.0 * value / dy.powi(2);
                            }
                        }
                    } else {
                        matrix[[i_l.current, i_l.neg[1]]] += 1.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dy.powi(2);
                        matrix[[i_l.current, i_l.pos[1]]] += 1.0 / dy.powi(2);
                    }
    
                    // Z direction
                    if i_z == 0 {
                        match boundary_conditions.pressure[2][0] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{k-1} = p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{-p_k + p_{k+1}}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.pos[2]]] += 1.0 / dz.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{k-1} + p_k) = value
                                // p_{k-1} = 2 * value - p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{2 * value - p_k - 2 p_k + p_{k+1}}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.pos[2]]] += 1.0 / dz.powi(2);
    
                                rhs[i_l.current] += -2.0 * value / dz.powi(2);
                            }
                        }
                    } else if i_z == nz - 1 {
                        match boundary_conditions.pressure[2][1] {
                            BoundaryCondition::ZeroGradient => {
                                // Principle: p_{k+1} = p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{-p_k + p_{k-1}}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -1.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.neg[2]]] += 1.0 / dz.powi(2);
                            },
                            BoundaryCondition::Value(value) => {
                                // Principle: p_face = value
                                // 0.5 * (p_{k+1} + p_k) = value
                                // p_{k+1} = 2 * value - p_k
                                // \frac{p_{k-1} - 2 p_k + p_{k+1}}{dz^2} = \frac{p_{k-1} - 2 p_k + 2 * value - p_k}{dz^2}
                                matrix[[i_l.current, i_l.current]] += -3.0 / dz.powi(2);
                                matrix[[i_l.current, i_l.neg[2]]] += 1.0 / dz.powi(2);
    
                                rhs[i_l.current] += -2.0 * value / dz.powi(2);
                            }
                        }
                    } else {
                        matrix[[i_l.current, i_l.neg[2]]] += 1.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.current]] += -2.0 / dz.powi(2);
                        matrix[[i_l.current, i_l.pos[2]]] += 1.0 / dz.powi(2);
                    }
                }
            }
        }
        
        (matrix, rhs)
    }
}

