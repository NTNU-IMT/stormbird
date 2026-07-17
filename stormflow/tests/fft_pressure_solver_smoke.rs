use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::pressure_solver::boundary_conditions::{PressureBoundaryCondition, PressureBoundaryConditions};
use stormflow::grid::Grid;
use stormflow::pressure_solver::multigrid_cpu::MultigridCPU;
use stormflow::pressure_solver::fft::FftCPU;
use stormflow::pressure_solver::settings::MultigridSettings;

/// Builds a synthetic RHS (not physically meaningful, just varied enough to exercise every mode
/// of the solve), mirroring `tests/gpu_pressure_solver_smoke.rs`'s `synthetic_rhs`.
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

/// The FFT solver is a *direct* solve of the boundary-folded discrete Poisson system, while
/// multigrid is iterative, so the reference here needs enough V-cycles to have converged well
/// past the tolerance we check against — otherwise a mismatch could just as easily be multigrid's
/// residual as an FFT solver bug.
fn converged_multigrid_settings() -> MultigridSettings {
    MultigridSettings {
        nr_v_cycles: 100,
        nr_smooth_iterations: 4
    }
}

fn assert_fft_matches_multigrid(grid: &Grid, boundary_conditions: &PressureBoundaryConditions) {
    let rhs = synthetic_rhs(grid);

    let mut multigrid_solver = MultigridCPU::new(grid, boundary_conditions, converged_multigrid_settings());
    multigrid_solver.rhs_at_levels[0].copy_from_slice(&rhs);
    multigrid_solver.solve();

    let mut fft_solver = FftCPU::new(grid, boundary_conditions);
    fft_solver.rhs.copy_from_slice(&rhs);
    fft_solver.solve();

    assert_eq!(multigrid_solver.solution.len(), fft_solver.solution.len());

    let [nx, ny, nz] = grid.interior_shape;

    let mut max_abs_diff: Float = 0.0;
    let mut max_abs_diff_flat = 0usize;
    let mut max_abs_diff_ghost_count = 0usize;

    let mut max_abs_diff_core: Float = 0.0;
    let mut max_abs_diff_core_flat = 0usize;

    for (flat, (a, b)) in multigrid_solver.solution.iter().zip(fft_solver.solution.iter()).enumerate() {
        assert!(a.is_finite(), "multigrid solution contains a non-finite value: {a}");
        assert!(b.is_finite(), "FFT solution contains a non-finite value: {b}");

        let diff = (a - b).abs();

        if diff > max_abs_diff {
            max_abs_diff = diff;
            max_abs_diff_flat = flat;
        }

        let [i, j, k] = grid.extended_indices_from_flat_index(flat);
        let ghost_count = [
            i == 0 || i == nx + 1,
            j == 0 || j == ny + 1,
            k == 0 || k == nz + 1,
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
    println!("max |multigrid - FFT| = {max_abs_diff} at ({i},{j},{k}), ghost_count={max_abs_diff_ghost_count}");

    let [ci, cj, ck] = grid.extended_indices_from_flat_index(max_abs_diff_core_flat);
    println!("max |multigrid - FFT| among core/interior cells only = {max_abs_diff_core} at ({ci},{cj},{ck})");

    assert!(max_abs_diff_core < 1e-3, "FFT solution diverges from multigrid solution (core cells) by {max_abs_diff_core}");
}

#[test]
fn fft_pressure_solver_matches_multigrid_with_up_direction_bc() {
    // Mirrors gpu_pressure_solver_smoke's cubic grid case: zero-gradient on every face except
    // zero-value on the top face (the practically relevant "free surface above, walls elsewhere"
    // configuration).
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 1.0]),
        [16, 16, 16]
    );

    let boundary_conditions = PressureBoundaryConditions::new_from_up_direction(SpatialVector([0.0, 1.0, 0.0]));

    assert_fft_matches_multigrid(&grid, &boundary_conditions);
}

#[test]
fn fft_pressure_solver_matches_multigrid_on_anisotropic_grid() {
    // Anisotropic (different cell counts per axis, exercising per-axis eigenvalue arrays of
    // different lengths), but unlike gpu_pressure_solver_smoke's [4, 128, 4] case, shaped so the
    // multigrid hierarchy still coarsens down through several levels. [4, 128, 4] coarsening stops
    // immediately (SMALLEST_NR_CELLS_FOR_COARSENING), leaving multigrid as plain weighted-Jacobi
    // on a 128-long thin domain — a fine same-algorithm GPU-vs-CPU check, but far too
    // slow-converging in a handful of V-cycles to serve as an independent reference for the FFT
    // solver here.
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 2.0, 1.0]),
        [16, 32, 16]
    );

    let boundary_conditions = PressureBoundaryConditions::new_from_up_direction(SpatialVector([0.0, 1.0, 0.0]));

    assert_fft_matches_multigrid(&grid, &boundary_conditions);
}

#[test]
fn fft_pressure_solver_matches_multigrid_with_arbitrary_bc_combination() {
    // Exercises all four per-axis transform kinds in a single solve (none of which
    // `new_from_up_direction` alone would produce): x-axis zero-gradient/zero-gradient,
    // y-axis zero-gradient/zero-value, z-axis zero-value/zero-gradient.
    let boundary_conditions = PressureBoundaryConditions::new_custom([
        [PressureBoundaryCondition::ZeroGradient, PressureBoundaryCondition::ZeroGradient],
        [PressureBoundaryCondition::ZeroGradient, PressureBoundaryCondition::ZeroValue],
        [PressureBoundaryCondition::ZeroValue, PressureBoundaryCondition::ZeroGradient],
    ]);

    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 1.0]),
        [16, 16, 16]
    );

    assert_fft_matches_multigrid(&grid, &boundary_conditions);
}

#[test]
fn fft_pressure_solver_matches_multigrid_with_all_zero_value_bc() {
    // All-Dirichlet: every axis uses the DST-II/DST-III pair, and there's no singular zero mode
    // to special-case (unlike the all-Neumann case).
    let boundary_conditions = PressureBoundaryConditions::new_custom([
        [PressureBoundaryCondition::ZeroValue, PressureBoundaryCondition::ZeroValue],
        [PressureBoundaryCondition::ZeroValue, PressureBoundaryCondition::ZeroValue],
        [PressureBoundaryCondition::ZeroValue, PressureBoundaryCondition::ZeroValue],
    ]);

    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 1.0]),
        [16, 16, 16]
    );

    assert_fft_matches_multigrid(&grid, &boundary_conditions);
}
