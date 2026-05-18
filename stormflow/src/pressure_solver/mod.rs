pub mod builder;
pub mod multigrid;
pub mod kernels;

use stormath::type_aliases::Float;

use multigrid::PressureSolverMultiGrid;

pub enum PressureSolver {
    Multigrid(PressureSolverMultiGrid)
}

impl PressureSolver {
    /// Solves the Poisson equation using the configured solver.
    /// 
    /// # Arguments
    /// * `pressure` - On input: initial guess on **extended** grid. On output: solution on **extended** grid.
    /// * `rhs` - Right-hand side on **interior** grid.
    pub fn solve(&mut self, pressure: &mut [Float], rhs: &[Float]) {
        match self {
            Self::Multigrid(solver) => solver.solve(pressure, rhs)
        };

        // Compute residual using stencil-based approach
        let grid = match self {
            Self::Multigrid(solver) => &solver.grids[0]
        };
        
        let mut residual = vec![0.0; rhs.len()];
        let residual_sum = kernels::compute_residual(grid, pressure, rhs, &mut residual);
        let avg_residual = residual_sum / rhs.len() as Float;

        println!("Residual sum: {}", avg_residual);
    }
}
