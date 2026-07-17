use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::pressure_solver::boundary_conditions::PressureBoundaryConditions;
use stormflow::grid::Grid;
use stormflow::pressure_solver::multigrid_cpu::MultigridCPU;
use stormflow::pressure_solver::multigrid_gpu::MultigridGPU;
use stormflow::pressure_solver::settings::MultigridSettings;

/// Builds a synthetic RHS (not physically meaningful, just varied enough to exercise the
/// restrict/prolongate/smoother chain across every multigrid level, including the coarsest one
/// which is small enough here to trigger the single-workgroup mega-kernel).
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

fn assert_gpu_matches_cpu(grid: &Grid) {
    let boundary_conditions = PressureBoundaryConditions::new_from_up_direction(SpatialVector([0.0, 1.0, 0.0]));
    let settings = MultigridSettings::default();

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
    println!("max |CPU - GPU| = {max_abs_diff} at ({i},{j},{k}), ghost_count={max_abs_diff_ghost_count}");

    let [ci, cj, ck] = grid.extended_indices_from_flat_index(max_abs_diff_core_flat);
    println!("max |CPU - GPU| among core/interior cells only = {max_abs_diff_core} at ({ci},{cj},{ck})");

    assert!(max_abs_diff_core < 1e-3, "GPU solution diverges from CPU solution (core cells) by {max_abs_diff_core}");
}

#[test]
fn gpu_pressure_solver_matches_cpu_with_coarse_mega_kernel() {
    // Coarsens 16 -> 8 -> 4 (extended 6^3 = 216 cells for the coarsest level), which fits in a
    // single workgroup (min guaranteed limit is 256), so this exercises the single-dispatch
    // coarse mega-kernel path.
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 1.0]),
        [16, 16, 16]
    );

    assert_gpu_matches_cpu(&grid);
}

#[test]
fn gpu_pressure_solver_matches_cpu_with_coarse_fallback() {
    // Anisotropic grid: coarsening stops immediately since two dimensions would drop below the
    // minimum (4 -> 2 is not > SMALLEST_NR_CELLS_FOR_COARSENING), leaving a single, large level
    // (4*128*4 = 2048 interior cells) that can't fit in one workgroup even on hardware that
    // grants the common 1024-thread max (GpuContext now requests the adapter's own limits rather
    // than wgpu's conservative 256-thread default) — exercises the fallback to the regular
    // per-iteration dispatch path.
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 8.0, 1.0]),
        [4, 128, 4]
    );

    assert_gpu_matches_cpu(&grid);
}
