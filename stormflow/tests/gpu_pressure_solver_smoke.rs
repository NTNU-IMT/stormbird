use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::pressure_solver::boundary_conditions::PressureBoundaryConditions;
use stormflow::grid::Grid;
use stormflow::grid::INTERIOR_OFFSET;
use stormflow::pressure_solver::multigrid_cpu::MultigridCPU;
use stormflow::pressure_solver::multigrid_gpu::MultigridGPU;
use stormflow::pressure_solver::multigrid_cpu::settings::{MultigridSettings, CoarsestLevelSolver};

/// Builds a synthetic RHS (not physically meaningful, just varied enough to exercise the
/// restrict/prolongate/smoother chain across every multigrid level, including the coarsest one).
fn synthetic_rhs(grid: &Grid) -> Vec<Float> {
    let n = grid.nr_interior_cells();
    let mut rhs = vec![0.0 as Float; n];

    for flat in 0..n {
        let [i, j, k] = grid.interior_indices_from_flat_index(flat);
        let value = ((i + 1) as Float).sin() + ((j + 1) as Float).cos() * 0.5 - ((k + 1) as Float) * 0.01;
        rhs[flat] = value;
    }

    rhs
}

/// Runs both solvers with the *same* `coarsest_level_solver`, so this checks that the GPU
/// solver's numerics reproduce the CPU solver's numerics for that algorithm — not that either one
/// converges to the "true" solution. `CoarsestLevelSolver::Jacobi` is only an approximation of the
/// coarsest level, so it does not necessarily agree with `CoarsestLevelSolver::Exact` (see
/// `gpu_pressure_solver_matches_cpu_with_jacobi_coarse_solve`'s single-level grid, where Jacobi's
/// limited iteration count can't converge at all).
fn assert_gpu_matches_cpu(grid: &Grid, coarsest_level_solver: CoarsestLevelSolver) {
    let boundary_conditions = PressureBoundaryConditions::new_from_up_direction(SpatialVector([0.0, 1.0, 0.0]));
    let settings = MultigridSettings{
        nr_smooth_iterations: 4,
        nr_v_cycles: 2,
        compute_residual_after_solve: true,
        coarsest_level_solver
    };

    let rhs = synthetic_rhs(&grid);

    let mut cpu_solver = MultigridCPU::new(&grid, &boundary_conditions, settings.clone());
    cpu_solver.rhs_at_levels[0].copy_from_slice(&rhs);
    cpu_solver.solve();

    let mut gpu_solver = MultigridGPU::new(&grid, &boundary_conditions, settings);
    gpu_solver.rhs.copy_from_slice(&rhs);
    gpu_solver.solve();

    assert_eq!(cpu_solver.solution.len(), gpu_solver.solution.len());

    let [nx, ny, nz] = grid.interior_shape;

    let mut max_abs_diff: Float = 0.0;
    let mut max_abs_diff_flat = 0usize;
    let mut max_abs_diff_ghost_count = 0usize;

    let mut max_abs_diff_core: Float = 0.0;
    let mut max_abs_diff_core_flat = 0usize;

    for (flat, (a, b)) in cpu_solver.solution.iter().zip(gpu_solver.solution.iter()).enumerate() {
        assert!(a.is_finite(), "CPU solution contains a non-finite value: {a}");
        assert!(b.is_finite(), "GPU solution contains a non-finite value: {b}");

        let diff = (a - b).abs();

        if diff > max_abs_diff {
            max_abs_diff = diff;
            max_abs_diff_flat = flat;
        }

        let [i, j, k] = grid.extended_indices_from_flat_index(flat);
        let ghost_count = [
            i < INTERIOR_OFFSET || i >= INTERIOR_OFFSET + nx,
            j < INTERIOR_OFFSET || j >= INTERIOR_OFFSET + ny,
            k < INTERIOR_OFFSET || k >= INTERIOR_OFFSET + nz,
        ].iter().filter(|&&b| b).count();

        if flat == max_abs_diff_flat {
            max_abs_diff_ghost_count = ghost_count;
        }

        if ghost_count <= 1 && diff > max_abs_diff_core {
            max_abs_diff_core = diff;
            max_abs_diff_core_flat = flat;
        }
    }

    let [i, j, k] = grid.extended_indices_from_flat_index(max_abs_diff_flat);
    println!("max |CPU - GPU| = {max_abs_diff} at ({i},{j},{k}), ghost_count={max_abs_diff_ghost_count}");

    let [ci, cj, ck] = grid.extended_indices_from_flat_index(max_abs_diff_core_flat);
    println!("max |CPU - GPU| among core/interior cells only = {max_abs_diff_core} at ({ci},{cj},{ck})");

    assert!(max_abs_diff_core < 1e-3, "GPU solution diverges from CPU solution (core cells) by {max_abs_diff_core}");
}

#[test]
fn gpu_pressure_solver_matches_cpu_with_exact_coarse_solve() {
    // Coarsens 16 -> 8 -> 4. Exercises `CoarsestLevelSolver::Exact` on the GPU solver: the
    // coarsest level's restricted RHS is read back to the CPU, solved exactly via
    // `build_poisson_matrix4`/`solve_gaussian_elimination` (the same machinery `MultigridCPU`
    // uses), and written back before prolongation.
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 1.0]),
        [16, 16, 16]
    );

    assert_gpu_matches_cpu(&grid, CoarsestLevelSolver::Exact);
}

#[test]
fn gpu_pressure_solver_matches_cpu_with_jacobi_coarse_solve() {
    // Anisotropic grid: coarsening stops immediately since two dimensions would drop below the
    // minimum (4 -> 2 is not > SMALLEST_NR_CELLS_FOR_COARSENING), leaving a single, large level
    // (4*128*4 = 2048 interior cells). Both solvers use `CoarsestLevelSolver::Jacobi` here, so
    // this checks that the GPU's per-iteration dispatch loop reproduces the CPU's Jacobi sweeps
    // bit-for-bit-ish — not that either one actually converges (16 iterations can't propagate
    // information across this grid's 128-cell axis, see `assert_gpu_matches_cpu`'s doc comment).
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 8.0, 1.0]),
        [4, 128, 4]
    );

    assert_gpu_matches_cpu(&grid, CoarsestLevelSolver::Jacobi);
}
