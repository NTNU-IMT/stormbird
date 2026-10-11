use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::geometry::{Geometry, WallGeometries};
use stormflow::geometry::analytical_shapes::Sphere;
use stormflow::grid::Grid;
use stormflow::pressure_solver::boundary_conditions::PressureBoundaryConditions;
use stormflow::pressure_solver::multigrid_cpu::MultigridCPU;
use stormflow::pressure_solver::multigrid_gpu::MultigridGPU;
use stormflow::pressure_solver::multigrid_cpu::settings::{
    MultigridSettings, CoarsestLevelSolver, ZeroGradientOnWalls
};
use stormflow::pressure_solver::multigrid_cpu::zero_gradient_stencils::ZeroGradientInterpolationOrder;

fn synthetic_rhs(grid: &Grid) -> Vec<Float> {
    (0..grid.nr_interior_cells()).map(|flat| {
        let [i, j, k] = grid.interior_indices_from_flat_index(flat);
        ((i + 1) as Float * 0.3).sin() + ((j + 1) as Float * 0.2).cos() * 0.5 - (k as Float) * 0.01
    }).collect()
}

/// Max absolute difference over the interior cells of two solutions on the extended grid
fn max_interior_diff(grid: &Grid, a: &[Float], b: &[Float]) -> Float {
    (0..grid.nr_interior_cells()).map(|flat| {
        let i = grid.flat_index_on_extended_grid_from_interior_indices(
            grid.interior_indices_from_flat_index(flat)
        );

        (a[i] - b[i]).abs()
    }).fold(0.0, Float::max)
}

struct Solutions {
    cpu: Vec<Float>,
    gpu: Vec<Float>,
}

fn solve(
    grid: &Grid,
    zero_gradient_on_walls: ZeroGradientOnWalls,
    coarsest_level_solver: CoarsestLevelSolver,
    slip_geometries: &[Geometry],
    no_slip_geometries: &[Geometry]
) -> Solutions {
    let boundary_conditions = PressureBoundaryConditions::new_from_up_direction(SpatialVector([0.0, 0.0, 1.0]));

    let settings = MultigridSettings {
        nr_v_cycles: 2,
        nr_smooth_iterations: 4,
        compute_residual_after_solve: false,
        coarsest_level_solver,
        zero_gradient_on_walls,
        zero_gradient_interpolation_order: ZeroGradientInterpolationOrder::Tricubic,
    };

    let rhs = synthetic_rhs(grid);

    let slip_walls = WallGeometries::new(slip_geometries.to_vec(), grid);
    let no_slip_walls = WallGeometries::new(no_slip_geometries.to_vec(), grid);

    let mut cpu_solver = MultigridCPU::new(
        grid, &boundary_conditions, settings.clone(), &slip_walls, &no_slip_walls
    );
    cpu_solver.rhs_at_levels[0].copy_from_slice(&rhs);
    cpu_solver.solve();

    let mut gpu_solver = MultigridGPU::new(
        grid, &boundary_conditions, settings, &slip_walls, &no_slip_walls
    );
    gpu_solver.rhs.copy_from_slice(&rhs);
    gpu_solver.solve();

    Solutions {
        cpu: cpu_solver.solution,
        gpu: gpu_solver.solution,
    }
}

/// One slip and one no-slip sphere, at different locations, so that the effect of each wall
/// selection can be told apart.
fn assert_zero_gradient_on_walls_works(coarsest_level_solver: CoarsestLevelSolver) {
    let grid = Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [32, 32, 32]);

    let slip_geometries = vec![
        Geometry::Sphere(Sphere { center: SpatialVector([0.3, 0.5, 0.5]), radius: 0.15 })
    ];
    let no_slip_geometries = vec![
        Geometry::Sphere(Sphere { center: SpatialVector([0.7, 0.5, 0.5]), radius: 0.15 })
    ];

    let mut cpu_solutions: Vec<(ZeroGradientOnWalls, Vec<Float>)> = Vec::new();

    for zero_gradient_on_walls in [
        ZeroGradientOnWalls::NotUsed,
        ZeroGradientOnWalls::SlipWallsOnly,
        ZeroGradientOnWalls::NoSlipWallsOnly,
        ZeroGradientOnWalls::AllWalls
    ] {
        let solutions = solve(
            &grid, zero_gradient_on_walls, coarsest_level_solver, &slip_geometries, &no_slip_geometries
        );

        let cpu_gpu_diff = max_interior_diff(&grid, &solutions.cpu, &solutions.gpu);

        println!("{coarsest_level_solver:?}/{zero_gradient_on_walls:?}: max |CPU - GPU| = {cpu_gpu_diff:e}");

        assert!(solutions.cpu.iter().all(|value| value.is_finite()));
        assert!(
            cpu_gpu_diff < 1e-5,
            "{zero_gradient_on_walls:?}: GPU solution diverges from CPU solution by {cpu_gpu_diff}"
        );

        cpu_solutions.push((zero_gradient_on_walls, solutions.cpu));
    }

    let not_used = &cpu_solutions[0].1;

    // Each variant that uses the condition must change the solution
    for (zero_gradient_on_walls, solution) in &cpu_solutions[1..] {
        let effect = max_interior_diff(&grid, solution, not_used);

        println!("{coarsest_level_solver:?}/{zero_gradient_on_walls:?}: effect = {effect:e}");

        assert!(effect > 1e-4, "{zero_gradient_on_walls:?} has no effect on the solution");
    }

    // Using all walls must be the same as treating both geometries as slip walls
    let both_as_slip: Vec<Geometry> = slip_geometries.iter().chain(&no_slip_geometries).cloned().collect();
    let all_walls_reference = solve(
        &grid, ZeroGradientOnWalls::SlipWallsOnly, coarsest_level_solver, &both_as_slip, &[]
    );

    assert_eq!(cpu_solutions[3].1, all_walls_reference.cpu);
}

#[test]
fn zero_gradient_on_walls_works_with_jacobi_coarse_solve() {
    assert_zero_gradient_on_walls_works(CoarsestLevelSolver::Jacobi);
}

#[test]
fn zero_gradient_on_walls_works_with_exact_coarse_solve() {
    assert_zero_gradient_on_walls_works(CoarsestLevelSolver::Exact);
}
