use oxifft::rdft::solvers::R2rSolver;
use rayon::prelude::*;
use stormath::type_aliases::Float;

use crate::{
    grid::Grid,
    pressure_solver::{boundary_conditions::PressureBoundaryConditions, multigrid_cpu::kernels as cpu_kernels}
};

pub mod transform;

use transform::{AxisTransformKind, TransformDirection, transform_along_axis};

/// Direct Poisson solver for the pressure equation, exploiting the fact that on a structured,
/// fixed-cell-size Cartesian grid the boundary-folded finite-difference Laplacian (see
/// `multigrid_cpu::kernels::jacobi::off_diagonal_sum`) is diagonalized exactly, per axis, by a
/// discrete cosine/sine transform selected by that axis' pair of boundary conditions — no
/// iteration needed, unlike the geometric multigrid solvers.
///
/// Uses the third-party `oxifft` crate for the underlying 1D DCT/DST transforms, parallelized
/// across the grid's independent lines via Rayon in `transform::transform_along_axis`; see
/// `transform::AxisTransformKind` for the boundary-condition-to-transform mapping and its
/// derivation.
///
/// Experimental alternative to `MultigridCPU`/`MultigridGPU`. Not wired into the simulation
/// loop's defaults; construct directly (see `tests/fft_pressure_solver_smoke.rs`).
pub struct FftCPU {
    pub grid: Grid,
    pub boundary_conditions: PressureBoundaryConditions,
    /// Right-hand side for the finest (only) level, on the **interior** grid. Written by the
    /// caller before calling `solve`, matching `MultigridGPU::rhs`'s convention. Left untouched by
    /// `solve` (the transform pipeline runs on a private scratch copy), so it also serves as the
    /// reference for the residual check printed at the end of `solve`.
    pub rhs: Vec<Float>,
    /// Solution for the finest (only) level, on the **extended** grid (matching
    /// `MultigridCPU::solution`/`MultigridGPU::solution`'s layout, since `Simulation::update_velocity`
    /// needs the boundary-extrapolated pressure one cell past the domain edge).
    pub solution: Vec<Float>,

    /// Scratch buffer (interior grid) the forward/divide/inverse pipeline runs on in place.
    working: Vec<Float>,

    /// Per-axis transform kind, selected from that axis' pair of boundary conditions.
    axis_transforms: [AxisTransformKind; 3],
    /// Per-axis cached `R2rSolver`, built once (each construction plans an FFT and builds twiddle
    /// tables) and reused for every line and every `solve()` call. Without this, calling
    /// `oxifft`'s convenience transform functions per line would re-plan from scratch on every
    /// single line — see `transform::AxisTransformKind::forward` for details.
    axis_solvers: [R2rSolver<Float>; 3],
    /// Per-axis eigenvalues of the boundary-folded 1D Laplacian stencil, indexed by mode.
    axis_eigenvalues: [Vec<Float>; 3],
    /// `1 / product(n_axis / 2)`, undoing the round-trip scale factor of `oxifft`'s
    /// forward-then-inverse transform pair (each axis contributes exactly `n_axis / 2`,
    /// independent of boundary condition combination — verified numerically against dense
    /// linear-algebra solves, see task history / `transform::AxisTransformKind`). Folded directly
    /// into the eigenvalue-division pass in `solve` rather than applied as a separate final pass,
    /// saving one full array traversal.
    inv_normalization: Float,
    /// Below this magnitude, an eigenvalue sum is treated as the singular (all-Neumann) zero
    /// mode: the corresponding spectral coefficient is pinned to zero rather than divided,
    /// fixing the arbitrary additive constant of a pure-Neumann pressure solve.
    singular_mode_threshold: Float,
}

impl FftCPU {
    pub fn new(grid: &Grid, boundary_conditions: &PressureBoundaryConditions) -> Self {
        let axis_transforms: [AxisTransformKind; 3] = std::array::from_fn(|axis| {
            AxisTransformKind::from_faces(
                boundary_conditions.condition(axis, 0),
                boundary_conditions.condition(axis, 1)
            )
        });

        let axis_solvers: [R2rSolver<Float>; 3] = std::array::from_fn(|axis| {
            AxisTransformKind::build_solver(grid.interior_shape[axis])
        });

        let axis_eigenvalues: [Vec<Float>; 3] = std::array::from_fn(|axis| {
            axis_transforms[axis].eigenvalues(grid.interior_shape[axis], grid.inv_cell_length_squared[axis])
        });

        let inv_normalization = 1.0 / grid.interior_shape.iter()
            .map(|&n| n as Float * 0.5)
            .product::<Float>();

        let nr_interior_cells = grid.nr_interior_cells();

        Self {
            grid: grid.clone(),
            boundary_conditions: boundary_conditions.clone(),
            rhs: vec![0.0; nr_interior_cells],
            solution: vec![0.0; grid.nr_extended_cells()],
            working: vec![0.0; nr_interior_cells],
            axis_transforms,
            axis_solvers,
            axis_eigenvalues,
            inv_normalization,
            singular_mode_threshold: grid.poisson_diagonal.abs() * 1e-6,
        }
    }

    /// Solves the Poisson equation directly via forward transform -> divide by eigenvalue ->
    /// inverse transform, per axis.
    ///
    /// # Note
    /// The caller is responsible for populating `self.rhs` (finest-level RHS on the interior
    /// grid) before calling this. On return, `self.solution` holds the solution on the extended
    /// grid, matching `MultigridCPU`/`MultigridGPU`'s layout.
    pub fn solve(&mut self) {
        let grid = &self.grid;
        let interior_shape = grid.interior_shape;
        let interior_stride = grid.interior_stride;

        self.working.copy_from_slice(&self.rhs);

        for axis in 0..3 {
            transform_along_axis(
                interior_shape,
                interior_stride,
                axis,
                &mut self.working,
                self.axis_transforms[axis],
                &self.axis_solvers[axis],
                TransformDirection::Forward
            );
        }

        let axis_eigenvalues = &self.axis_eigenvalues;
        let singular_mode_threshold = self.singular_mode_threshold;
        let inv_normalization = self.inv_normalization;

        self.working
            .par_iter_mut()
            .enumerate()
            .for_each(|(flat_interior, value)| {
                let [i, j, k] = grid.interior_indices_from_flat_index(flat_interior);

                let lambda = axis_eigenvalues[0][i] + axis_eigenvalues[1][j] + axis_eigenvalues[2][k];

                if lambda.abs() < singular_mode_threshold {
                    *value = 0.0;
                } else {
                    // Folds the post-inverse-transform normalization (see `inv_normalization`'s
                    // doc comment) directly into this division, since scaling by a constant
                    // commutes with the (linear) inverse transform applied afterwards — saving a
                    // separate full-array pass at the end.
                    *value *= inv_normalization / lambda;
                }
            });

        for axis in (0..3).rev() {
            transform_along_axis(
                interior_shape,
                interior_stride,
                axis,
                &mut self.working,
                self.axis_transforms[axis],
                &self.axis_solvers[axis],
                TransformDirection::Inverse
            );
        }

        // Materialize the extended layout `update_velocity`/`export_fields_as_vtk` expect, exactly
        // as `MultigridCPU::solve` does for its own interior result.
        let working = &self.working;
        let solution_ptr = self.solution.as_mut_ptr() as usize;

        (0..working.len())
            .into_par_iter()
            .for_each(|flat_interior| {
                let interior_indices = grid.interior_indices_from_flat_index(flat_interior);
                let flat_extended = grid.flat_index_on_extended_grid_from_interior_indices(interior_indices);

                // Safety: flat_extended is unique per flat_interior (bijective interior <->
                // extended mapping), so each thread writes a distinct index.
                unsafe {
                    let ptr = solution_ptr as *mut Float;
                    *ptr.add(flat_extended) = working[flat_interior];
                }
            });

        self.boundary_conditions.set_ghost_cells(grid, &mut self.solution);

        let avg_residual = cpu_kernels::compute_residual(grid, &self.solution, &self.rhs);
        println!("Residual sum: {}", avg_residual);
    }
}
