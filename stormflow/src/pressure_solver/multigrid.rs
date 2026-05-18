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
    pub grids: Vec<Grid>,
    pub matrices: Vec<SparseMatrix::<MATRIX_ROW_LENGTH>>,
    pub solver_settings: IterativeSolverSettings,
    pub x_at_levels: Vec<Vec<Float>>,
    pub x_at_levels_work: Vec<Vec<Float>>,
    pub rhs_at_levels: Vec<Vec<Float>>,
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

        let mut x_at_levels: Vec<Vec<Float>> = Vec::new();
        let mut rhs_at_levels: Vec<Vec<Float>> = Vec::new();

        let mut current_grid = grid.clone();

        let mut grids: Vec<Grid> = Vec::new();

        while grid_can_get_coarser {
            let (mut matrix, _) = PressureSolver::poisson_matrix_and_rhs(
                &current_grid, 
                boundary_conditions
            );

            grids.push(current_grid.clone());

            matrix.set_diagonal_data();

            matrices.push(matrix);

            let interior_shape_current = current_grid.interior_shape;

            let nr_cells_total = interior_shape_current[0] * 
                interior_shape_current[1] *
                interior_shape_current[2];

            x_at_levels.push(
                vec![0.0; nr_cells_total]
            );

            rhs_at_levels.push(
                vec![0.0; nr_cells_total]
            );

            if interior_shape_current[0] % 2 != 0 ||
                interior_shape_current[1] % 2 != 0 ||
                interior_shape_current[2] % 2 != 0 {
                    grid_can_get_coarser = false
            } else {
                let coarser_grid = current_grid.coarsened();
                let interior_shape = coarser_grid.interior_shape;
    
                if interior_shape[0] > SMALLEST_NR_CELLS && 
                    interior_shape[1] > SMALLEST_NR_CELLS && 
                    interior_shape[2] > SMALLEST_NR_CELLS {
                    current_grid = coarser_grid
                } else {
                    grid_can_get_coarser = false
                }
            }            
        }

        let x_at_levels_work = x_at_levels.clone();

        Self {
            grids,
            matrices,
            solver_settings,
            x_at_levels,
            x_at_levels_work,
            rhs_at_levels,
        }        
    }

    /// Computes residuals at the fine level and restricts them to the coarse level in a single fused pass.
    /// 
    /// This avoids storing the full residual array at the fine level, reducing memory traffic.
    /// For each coarse cell, computes residuals for its 8 fine children on-the-fly and averages them.
    /// 
    /// The residual for fine cell i is: r_i = rhs_i - (A * x)_i
    /// 
    /// # Safety
    /// Uses unsafe pointer access to enable parallel writes. This is safe because each
    /// coarse cell index is processed exactly once, so there are no data races.
    pub fn compute_residual_and_restrict(&mut self, fine_level: usize) {
        let coarse_level = fine_level + 1;
        let weight = 1.0 / 8.0;
        
        let nr_coarse_cells = self.grids[coarse_level].nr_interior_cells();
        let [_, ny_f, nz_f] = self.grids[fine_level].interior_shape;
        
        // Precompute strides for the fine grid
        let stride_k: usize = 1;
        let stride_j: usize = nz_f;
        let stride_i: usize = ny_f * nz_f;
        
        // Get pointers and references for parallel access
        let rhs_coarse_ptr = self.rhs_at_levels[coarse_level].as_mut_ptr() as usize;
        let x_fine = &self.x_at_levels[fine_level];
        let rhs_fine = &self.rhs_at_levels[fine_level];
        let matrix = &self.matrices[fine_level];
        let coarse_grid = &self.grids[coarse_level];
        
        // Offsets for the 8 fine children of a coarse cell (unrolled 2x2x2 stencil)
        let offsets = [
            0,
            stride_k,
            stride_j,
            stride_j + stride_k,
            stride_i,
            stride_i + stride_k,
            stride_i + stride_j,
            stride_i + stride_j + stride_k,
        ];
        
        (0..nr_coarse_cells)
            .into_par_iter()
            .for_each(|flat_index_coarse| {
                let [i_c, j_c, k_c] = coarse_grid.interior_indices_from_flat_index(flat_index_coarse);
                
                // Compute base index for the fine grid cell at (2*i_c, 2*j_c, 2*k_c)
                let base = (2 * i_c) * stride_i + (2 * j_c) * stride_j + (2 * k_c);
                
                let mut restricted_value: Float = 0.0;
                
                // For each of the 8 fine children, compute residual and accumulate
                for &offset in &offsets {
                    let fine_idx = base + offset;
                    
                    // Compute (A * x)[fine_idx] - the matrix-vector product for this row
                    let (row_values, row_cols) = matrix.row_entries(fine_idx);
                    let mut ax_i: Float = 0.0;
                    for k in 0..row_values.len() {
                        ax_i += row_values[k] * x_fine[row_cols[k]];
                    }
                    
                    // Compute residual: r_i = rhs_i - (A * x)_i
                    let residual_i = rhs_fine[fine_idx] - ax_i;
                    
                    restricted_value += weight * residual_i;
                }
                
                // Write result using unsafe pointer access
                // Safety: Each flat_index_coarse is unique, so no data races occur
                unsafe {
                    let ptr = rhs_coarse_ptr as *mut Float;
                    *ptr.add(flat_index_coarse) = restricted_value;
                }
            });
    }

    /// Prolongates (interpolates) the correction from a coarser grid level to a finer level
    /// using trilinear interpolation, and adds it directly to x_at_levels[fine_level].
    /// 
    /// This fuses the prolongation and correction addition into a single pass, avoiding
    /// the need for a separate corrections buffer and reducing memory traffic.
    /// 
    /// Each fine cell value is updated as: x_fine += interpolated(x_coarse)
    /// 
    /// # Safety
    /// Uses unsafe pointer access to enable parallel read-modify-write. This is safe because
    /// each fine cell index is processed exactly once, so there are no data races.
    pub fn prolongate_and_correct(&mut self, fine_level: usize) {
        let coarse_level = fine_level + 1;
        
        let [nx_c, ny_c, nz_c] = self.grids[coarse_level].interior_shape;
        let nr_fine_cells = self.grids[fine_level].nr_interior_cells();
        
        // Get raw pointers for parallel access
        let x_fine_ptr = self.x_at_levels[fine_level].as_mut_ptr() as usize;
        let coarse_values = &self.x_at_levels[coarse_level];
        let fine_grid = &self.grids[fine_level];
        let coarse_grid = &self.grids[coarse_level];
        
        (0..nr_fine_cells)
            .into_par_iter()
            .for_each(|flat_index_fine| {
                let [i_f, j_f, k_f] = fine_grid.interior_indices_from_flat_index(flat_index_fine);
                
                // Fine cell center position in "coarse cell units"
                // Fine cell i_f has center at (i_f + 0.5) * dx_f = (i_f + 0.5) * dx_c / 2
                // In coarse cell units (where coarse cell j has center at j + 0.5),
                // the fine cell center is at: (i_f + 0.5) / 2 = i_f/2 + 0.25
                // We want position relative to coarse cell centers at j + 0.5,
                // so xi = (i_f + 0.5) / 2 - 0.5 = i_f/2 - 0.25
                let xi = (i_f as Float) / 2.0 - 0.25;
                let eta = (j_f as Float) / 2.0 - 0.25;
                let zeta = (k_f as Float) / 2.0 - 0.25;
                
                // Find the "lower" coarse cell index for interpolation
                let i_c_base = (xi.floor() as isize).max(0).min((nx_c - 1) as isize) as usize;
                let j_c_base = (eta.floor() as isize).max(0).min((ny_c - 1) as isize) as usize;
                let k_c_base = (zeta.floor() as isize).max(0).min((nz_c - 1) as isize) as usize;
                
                // Local coordinates within the interpolation stencil [0, 1]
                let sx = (xi - (i_c_base as Float)).clamp(0.0, 1.0);
                let sy = (eta - (j_c_base as Float)).clamp(0.0, 1.0);
                let sz = (zeta - (k_c_base as Float)).clamp(0.0, 1.0);
                
                // Trilinear interpolation weights
                let wx = [1.0 - sx, sx];
                let wy = [1.0 - sy, sy];
                let wz = [1.0 - sz, sz];
                
                let mut correction_value: Float = 0.0;
                
                for di in 0..2 {
                    for dj in 0..2 {
                        for dk in 0..2 {
                            let i_c = (i_c_base + di).min(nx_c - 1);
                            let j_c = (j_c_base + dj).min(ny_c - 1);
                            let k_c = (k_c_base + dk).min(nz_c - 1);
                            
                            let weight = wx[di] * wy[dj] * wz[dk];
                            
                            let flat_index_coarse = coarse_grid.flat_index_on_interior_grid([i_c, j_c, k_c]);
                            
                            correction_value += weight * coarse_values[flat_index_coarse];
                        }
                    }
                }
                
                // Add correction directly to x using unsafe pointer access
                // Safety: Each flat_index_fine is unique, so no data races occur
                unsafe {
                    let ptr = x_fine_ptr as *mut Float;
                    *ptr.add(flat_index_fine) += correction_value;
                }
            });
    }
    
    pub fn perform_v_cycle(
        &mut self,
        rhs: &[Float]
    ) {
        let nr_grids = self.matrices.len();

        self.rhs_at_levels[0].copy_from_slice(rhs);

        let nr_iterations = self.solver_settings.max_number_of_iterations;
        let omega = self.solver_settings.jacobi_weight;

        // Smooth and restrict down to the coarsest level
        for i_g in 0..nr_grids-1 {
            if i_g > 0 {
                self.x_at_levels[i_g].fill(0.0);
            }

            self.matrices[i_g].solve_jacobi_into(
                &self.rhs_at_levels[i_g], 
                &mut self.x_at_levels[i_g], 
                &mut self.x_at_levels_work[i_g], 
                nr_iterations,
                omega
            );
            
            // Fused: compute residuals and restrict to coarser level in one pass
            self.compute_residual_and_restrict(i_g);
        }

        // Solve at the coarsest level
        // Note: We reuse the previous values as initial guess (warm start)
        // since we do more iterations here and the RHS changes gradually
        self.matrices[nr_grids-1].solve_jacobi_into(
            &self.rhs_at_levels[nr_grids-1],
            &mut self.x_at_levels[nr_grids-1],
            &mut self.x_at_levels_work[nr_grids-1],
            nr_iterations * 2,
            omega
        );
        
        // Prolongate and smooth back up
        for i_g in (0..nr_grids-1).rev() {
            // Fused: prolongate correction from coarse level and add directly to x
            self.prolongate_and_correct(i_g);

            self.matrices[i_g].solve_jacobi_into(
                &self.rhs_at_levels[i_g], 
                &mut self.x_at_levels[i_g], 
                &mut self.x_at_levels_work[i_g], 
                nr_iterations,
                omega
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
