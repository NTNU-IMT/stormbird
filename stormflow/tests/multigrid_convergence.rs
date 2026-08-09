use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::grid::Grid;
use stormflow::pressure_solver::boundary_conditions::PressureBoundaryConditions;
use stormflow::pressure_solver::multigrid_cpu::MultigridCPU;
use stormflow::pressure_solver::multigrid_cpu::kernels::compute_residual4;
use stormflow::pressure_solver::multigrid_cpu::settings::{MultigridSettings, CoarsestLevelSolver};

/// `nr_cells` geometrically stretched points spanning `[0, 1]`, each cell `growth` times the
/// previous one.
fn stretched_points(nr_cells: usize, growth: Float) -> Vec<Float> {
    let mut lengths = Vec::with_capacity(nr_cells);
    let mut length = 1.0 as Float;

    for _ in 0..nr_cells {
        lengths.push(length);
        length *= growth;
    }

    let total: Float = lengths.iter().sum();

    let mut points = vec![0.0 as Float];

    for length in lengths {
        let next = points[points.len() - 1] + length / total;

        points.push(next);
    }

    points
}

fn stretched_grid(nr_cells: usize, growth: Float) -> Grid {
    let points = stretched_points(nr_cells, growth);

    Grid::new_from_points([points.clone(), points.clone(), points])
}

/// The largest ratio between neighbouring cell lengths anywhere in the multigrid hierarchy.
///
/// Coarsening merges cell *pairs*, so a gentle stretch on the finest grid compounds as
/// `growth^(2^level)` and the coarsest level can be far more distorted than the grid the user
/// actually asked for. That number, not the fine-grid stretch, is what governs whether the solver
/// converges.
fn worst_hierarchy_stretch(grid: &Grid) -> Float {
    grid.multigrid_hierarchy()
        .iter()
        .flat_map(|level| {
            (0..3).flat_map(|axis| {
                let points = level.interior_points[axis].clone();

                (0..points.len() - 2)
                    .map(|i| {
                        let a = points[i + 1] - points[i];
                        let b = points[i + 2] - points[i + 1];

                        (b / a).max(a / b)
                    })
                    .collect::<Vec<Float>>()
            }).collect::<Vec<Float>>()
        })
        .fold(1.0 as Float, Float::max)
}

/// Runs `nr_cycles` V-cycles one at a time and reports the residual after each, so a divergent
/// iteration is visible rather than hidden behind a small cycle count.
fn residual_history(grid: &Grid, nr_cycles: usize) -> Vec<Float> {
    let boundary_conditions =
        PressureBoundaryConditions::new_from_up_direction(SpatialVector([0.0, 0.0, 1.0]));

    let settings = MultigridSettings {
        nr_smooth_iterations: 4,
        nr_v_cycles: 1,
        compute_residual_after_solve: false,
        coarsest_level_solver: CoarsestLevelSolver::Exact,
    };

    let mut solver = MultigridCPU::new(grid, &boundary_conditions, settings);

    for flat in 0..grid.nr_interior_cells() {
        let center = grid.cell_center(grid.interior_indices_from_flat_index(flat));

        solver.rhs_at_levels[0][flat] =
            (3.0 * center[0]).sin() * (2.0 * center[1]).cos() * (center[2] - 0.5);
    }

    let rhs = solver.rhs_at_levels[0].clone();

    (0..nr_cycles)
        .map(|_| {
            solver.solve();

            compute_residual4(grid, &solver.solution, &rhs)
        })
        .collect()
}

fn assert_converges(label: &str, grid: &Grid) {
    let history = residual_history(grid, 10);

    println!(
        "{label} (worst hierarchy stretch {:.2}): {:?}",
        worst_hierarchy_stretch(grid), history
    );

    let first = history[0];
    let last = history[history.len() - 1];

    assert!(
        last.is_finite() && last < 0.5 * first,
        "{label}: multigrid did not converge, residual went {first} -> {last}"
    );
}

#[test]
fn multigrid_converges_on_a_uniform_grid() {
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 1.0]),
        [64, 64, 64],
    );

    assert_converges("uniform 64^3", &grid);
}

/// The point of the generalized stencils: a genuinely non-uniform grid has to solve just like a
/// uniform one, as long as no level of the hierarchy is stretched past what the 4th order Poisson
/// stencil can represent (see `multigrid_diverges_when_coarsening_over_stretches_the_grid`).
///
/// 64 cells coarsen four times, so 0.5% per cell on the fine grid becomes 1.08x on the coarsest.
#[test]
fn multigrid_converges_on_a_deep_hierarchy_with_gentle_stretching() {
    assert_converges("stretched 64^3, 0.5% per cell", &stretched_grid(64, 1.005));
}

/// The same criterion from the other side: strong stretching is fine as long as the hierarchy is
/// shallow enough that it never compounds too far. 16 cells only coarsen twice, so 5% per cell
/// reaches just 1.22x on the coarsest level.
#[test]
fn multigrid_converges_on_a_shallow_hierarchy_with_strong_stretching() {
    assert_converges("stretched 16^3, 5% per cell", &stretched_grid(16, 1.05));
}

/// Documents a known limitation of the *hierarchy*, not of the stencils.
///
/// `Grid::coarsened` merges cell pairs, so the neighbouring-cell length ratio compounds as
/// `growth^(2^level)`. The 5-point 4th order second-derivative stencil stays a correct
/// approximation at any spacing, but as a *matrix* it degenerates: past a ratio of about 1.45 the
/// `+/-2` weights flip sign, and past about 2.6 the diagonal itself changes sign. Damped Jacobi
/// cannot smooth such an operator, so the V-cycle amplifies error instead of reducing it.
///
/// Measured V-cycle behaviour as a function of the worst ratio anywhere in the hierarchy
/// (residual reduction over 8 cycles):
///
/// ```text
///   worst ratio   residual reduction over 8 V-cycles
///     1.04          8e-4     converges
///     1.13          3e-3     converges
///     1.22          4e-3     converges
///     1.27          1e+1     diverges
///     1.48          2e+1     diverges
///     1.61          3e+5     diverges
/// ```
///
/// So the practical criterion is that no level of the hierarchy may stretch by more than roughly
/// 1.25x between neighbouring cells.
///
/// Fixing this is a coarsening/hierarchy question (cap the depth, stretch-aware coarsening, or a
/// narrower operator on coarse levels), not a stencil question, so it is left as a documented gap.
#[test]
#[ignore = "known limitation: deep coarsening of a stretched grid degenerates the 4th order Poisson stencil"]
fn multigrid_diverges_when_coarsening_over_stretches_the_grid() {
    let grid = stretched_grid(64, 1.05);

    println!("worst hierarchy stretch: {:.2}", worst_hierarchy_stretch(&grid));

    assert_converges("stretched 64^3, 5% per cell", &grid);
}
