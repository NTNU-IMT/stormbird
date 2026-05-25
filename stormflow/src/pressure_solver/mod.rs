pub mod builder;
pub mod kernels;

use stormath::type_aliases::Float;

use rayon::prelude::*;

use std::time::Instant;

use crate::{
    boundary_conditions::pressure::PressureBoundaryConditions, 
    grid::Grid
};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureSolverSettings {
    #[serde(default="PressureSolverSettings::default_nr_v_cycles")]
    pub nr_v_cycles: usize,
    #[serde(default="PressureSolverSettings::default_nr_smooth_iterations")]
    pub nr_smooth_iterations: usize,
    #[serde(default="PressureSolverSettings::default_jacobi_weight")]
    pub jacobi_weight: Float
}

impl PressureSolverSettings {
    pub fn default_nr_v_cycles() -> usize {2}
    pub fn default_nr_smooth_iterations() -> usize {4}
    pub fn default_jacobi_weight() -> Float {2.0/3.0}
}

impl Default for PressureSolverSettings {
    fn default() -> Self {
        Self {
            nr_v_cycles: Self::default_nr_v_cycles(),
            nr_smooth_iterations: Self::default_nr_smooth_iterations(),
            jacobi_weight: Self::default_jacobi_weight()
        }
    }
}

const RESTRICT_WEIGHT: Float = 1.0 / 8.0;

const RESTRICT_CHILD_OFFSETS: [(usize, usize, usize); 8] = [
    (0, 0, 0),
    (0, 0, 1),
    (0, 1, 0),
    (0, 1, 1),
    (1, 0, 0),
    (1, 0, 1),
    (1, 1, 0),
    (1, 1, 1),
];

pub struct PressureSolverMultiGrid {
    pub grids: Vec<Grid>,
    pub boundary_conditions: PressureBoundaryConditions,
    pub solver_settings: PressureSolverSettings,
    /// Solution values at each grid level. Stored on the **extended** grid (includes ghost cells).
    pub x_at_levels: Vec<Vec<Float>>,
    /// Work buffer for Jacobi iterations. Stored on the **extended** grid.
    pub x_at_levels_work: Vec<Vec<Float>>,
    /// Right-hand side at each grid level. Stored on the **interior** grid.
    pub rhs_at_levels: Vec<Vec<Float>>,
}

const SMALLEST_NR_CELLS: usize = 2;

impl PressureSolverMultiGrid {
    pub fn new(
        grid: &Grid, 
        boundary_conditions: &PressureBoundaryConditions, 
        solver_settings: PressureSolverSettings
    ) -> Self {
        let mut grid_can_get_coarser = true;

        let mut x_at_levels: Vec<Vec<Float>> = Vec::new();
        let mut rhs_at_levels: Vec<Vec<Float>> = Vec::new();

        let mut current_grid = grid.clone();

        let mut grids: Vec<Grid> = Vec::new();

        while grid_can_get_coarser {
            grids.push(current_grid.clone());

            // x_at_levels uses extended grid (for ghost cells / boundary conditions)
            x_at_levels.push(
                vec![0.0; current_grid.nr_extended_cells()]
            );

            // rhs_at_levels uses interior grid (RHS is defined on interior only)
            rhs_at_levels.push(
                vec![0.0; current_grid.nr_interior_cells()]
            );

            let interior_shape_current = current_grid.interior_shape;

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
            boundary_conditions: boundary_conditions.clone(),
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
    /// where A is the discrete Laplacian (Poisson matrix).
    /// 
    /// # Grid layout
    /// - `x_fine` is on the **extended** grid (includes ghost cells)
    /// - `rhs_fine` is on the **interior** grid
    /// - `rhs_coarse` (output) is on the **interior** grid
    /// 
    /// # Safety
    /// Uses unsafe pointer access to enable parallel writes. This is safe because each
    /// coarse cell index is processed exactly once, so there are no data races.
    pub fn compute_residual_and_restrict(&mut self, fine_level: usize) {
        let start_time = Instant::now();
        
        let coarse_level = fine_level + 1;
        
        let fine_grid = &self.grids[fine_level];
        let coarse_grid = &self.grids[coarse_level];
        
        let nr_coarse_interior_cells = coarse_grid.nr_interior_cells();
        
        // Precompute stencil coefficients for the fine grid
        let inv_dx2 = fine_grid.inv_cell_length[0].powi(2);
        let inv_dy2 = fine_grid.inv_cell_length[1].powi(2);
        let inv_dz2 = fine_grid.inv_cell_length[2].powi(2);
        let diag = -2.0 * (inv_dx2 + inv_dy2 + inv_dz2);
        
        // Strides for the fine extended grid
        let [stride_x_fine, stride_y_fine, _] = fine_grid.extended_stride;
        
        // Get pointers and references for parallel access
        let rhs_coarse_ptr = self.rhs_at_levels[coarse_level].as_mut_ptr() as usize;
        let x_fine = &self.x_at_levels[fine_level];
        let rhs_fine = &self.rhs_at_levels[fine_level];
        
        // Offsets for the 8 fine children of a coarse cell (2x2x2 stencil)
        // These are in terms of interior indices
        
        
        (0..nr_coarse_interior_cells)
            .into_par_iter()
            .for_each(|flat_index_coarse_interior| {
                // Get coarse interior indices
                let [i_c, j_c, k_c] = coarse_grid.interior_indices_from_flat_index(flat_index_coarse_interior);
                
                // Base fine interior indices (each coarse cell maps to 2x2x2 fine cells)
                let base_i_f = 2 * i_c;
                let base_j_f = 2 * j_c;
                let base_k_f = 2 * k_c;
                
                let mut restricted_value: Float = 0.0;
                
                // For each of the 8 fine children, compute residual and accumulate
                for &(di, dj, dk) in &RESTRICT_CHILD_OFFSETS {
                    // Fine interior indices
                    let i_f = base_i_f + di;
                    let j_f = base_j_f + dj;
                    let k_f = base_k_f + dk;
                    
                    // Flat interior index for rhs_fine
                    let flat_fine_interior = fine_grid.flat_index_on_interior_grid([i_f, j_f, k_f]);
                    
                    // Extended indices for x_fine (offset by 1 for ghost layer)
                    let ei_f = i_f + 1;
                    let ej_f = j_f + 1;
                    let ek_f = k_f + 1;
                    let idx_fine_extended = ei_f * stride_x_fine + ej_f * stride_y_fine + ek_f;
                    
                    // Compute (A * x)[fine_idx] using the Laplacian stencil
                    let ax_i = kernels::apply_laplacian_stencil(
                        x_fine,
                        idx_fine_extended,
                        inv_dx2, inv_dy2, inv_dz2, diag,
                        stride_x_fine, stride_y_fine
                    );
                    
                    // Compute residual: r_i = rhs_i - (A * x)_i
                    let residual_i = rhs_fine[flat_fine_interior] - ax_i;
                    
                    restricted_value += RESTRICT_WEIGHT * residual_i;
                }
                
                // Write result using unsafe pointer access
                // Safety: Each flat_index_coarse_interior is unique, so no data races occur
                unsafe {
                    let ptr = rhs_coarse_ptr as *mut Float;
                    *ptr.add(flat_index_coarse_interior) = restricted_value;
                }
            });

        println!("Restrict time: {:.?}", start_time.elapsed());
    }

    /// Prolongates (interpolates) the correction from a coarser grid level to a finer level
    /// using trilinear interpolation, and adds it directly to x_at_levels[fine_level].
    /// 
    /// This fuses the prolongation and correction addition into a single pass, avoiding
    /// the need for a separate corrections buffer and reducing memory traffic.
    /// 
    /// Each fine cell value is updated as: x_fine += interpolated(x_coarse)
    /// 
    /// # Grid layout
    /// - `x_fine` is on the **extended** grid
    /// - `x_coarse` is on the **extended** grid
    /// - Only interior cells of x_fine are updated
    /// 
    /// # Safety
    /// Uses unsafe pointer access to enable parallel read-modify-write. This is safe because
    /// each fine cell index is processed exactly once, so there are no data races.
    pub fn prolongate_and_correct(&mut self, fine_level: usize) {
        let start_time = Instant::now();
        
        let coarse_level = fine_level + 1;
        
        let fine_grid = &self.grids[fine_level];
        let coarse_grid = &self.grids[coarse_level];
        
        let [nx_c, ny_c, nz_c] = coarse_grid.interior_shape;
        let [nx_f, ny_f, nz_f] = fine_grid.interior_shape;
        
        // Get raw pointers for parallel access
        let x_fine_ptr = self.x_at_levels[fine_level].as_mut_ptr() as usize;
        let coarse_values = &self.x_at_levels[coarse_level];

        (0..nx_f).into_par_iter().for_each(|i_f| {
            for j_f in 0..ny_f {
                for k_f in 0..nz_f {
                    // Fine extended index (for writing to x_fine)
                    let idx_fine_extended = (i_f + 1) * fine_grid.extended_stride[0] + (j_f + 1) * fine_grid.extended_stride[1] + (k_f + 1);
                    
                    // Fine cell center position in "coarse cell units"
                    // Fine cell i_f has center at (i_f + 0.5) * dx_f = (i_f + 0.5) * dx_c / 2
                    // In coarse cell units (where coarse cell j has center at j + 0.5),
                    // the fine cell center is at: (i_f + 0.5) / 2 = i_f/2 + 0.25
                    // We want position relative to coarse cell centers at j + 0.5,
                    // so xi = (i_f + 0.5) / 2 - 0.5 = i_f/2 - 0.25
                    let xi = (i_f as Float) * 0.5 - 0.25;
                    let eta = (j_f as Float) * 0.5 - 0.25;
                    let zeta = (k_f as Float) * 0.5 - 0.25;
                    
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
                                
                                // Coarse extended index (offset by 1 for ghost layer)
                                let idx_coarse_extended = (i_c + 1) * coarse_grid.extended_stride[0] 
                                                        + (j_c + 1) * coarse_grid.extended_stride[1] 
                                                        + (k_c + 1);
                                
                                correction_value += weight * coarse_values[idx_coarse_extended];
                            }
                        }
                    }
                    
                    unsafe {
                        let ptr = x_fine_ptr as *mut Float;
                        *ptr.add(idx_fine_extended) += correction_value;
                    }
                }
            }
        });

        if fine_level == 0 {
            println!("Prolongate and correct time: {:.?}", start_time.elapsed());
        }
    }
    
    pub fn perform_v_cycle(&mut self) {
        let nr_grids = self.grids.len();

        let nr_iterations = self.solver_settings.nr_smooth_iterations;
        let omega = self.solver_settings.jacobi_weight;

        // Smooth and restrict down to the coarsest level
        for i_g in 0..nr_grids-1 {
            if i_g > 0 {
                self.x_at_levels[i_g].fill(0.0);
            }

            kernels::poisson_jacobi_smoother(
                &self.grids[i_g],
                &self.boundary_conditions,
                &self.rhs_at_levels[i_g], 
                &mut self.x_at_levels[i_g], 
                &mut self.x_at_levels_work[i_g], 
                nr_iterations,
                omega
            );

            self.compute_residual_and_restrict(i_g);
        }

        self.x_at_levels[nr_grids - 1].fill(0.0);

        kernels::poisson_jacobi_smoother(
            &self.grids[nr_grids-1], 
            &self.boundary_conditions,
            &self.rhs_at_levels[nr_grids-1], 
            &mut self.x_at_levels[nr_grids-1], 
            &mut self.x_at_levels_work[nr_grids-1], 
            nr_iterations * 4,
            omega
        );
        
        // Prolongate and smooth back up
        for i_g in (0..nr_grids-1).rev() {
            self.prolongate_and_correct(i_g);

            kernels::poisson_jacobi_smoother(
                &self.grids[i_g],
                &self.boundary_conditions,
                &self.rhs_at_levels[i_g], 
                &mut self.x_at_levels[i_g], 
                &mut self.x_at_levels_work[i_g], 
                nr_iterations,
                omega
            );
        }
    }

    /// Solves the Poisson equation using multigrid V-cycles.
    /// 
    /// # Arguments
    /// * `solution` - On input: initial guess on **extended** grid. On output: solution on **extended** grid.
    /// * `rhs` - Right-hand side on **interior** grid.
    /// 
    /// # Note
    /// The caller is responsible for setting ghost cell values in `solution` before calling this,
    /// and the ghost cells in the returned solution will have been modified during iteration.
    /// You may need to re-apply boundary conditions after calling this function.
    pub fn solve(&mut self) {
        for _ in 0..self.solver_settings.nr_v_cycles {
            self.perform_v_cycle();
        }

        // Compute residual using stencil-based approach        
        let avg_residual = kernels::compute_residual(
            &self.grids[0], 
            &self.x_at_levels[0], 
            &self.rhs_at_levels[0]
        );

        println!("Residual sum: {}", avg_residual);
    }
}
