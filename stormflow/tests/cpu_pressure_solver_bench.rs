use std::time::Instant;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::pressure_solver::boundary_conditions::PressureBoundaryConditions;
use stormflow::grid::Grid;
use stormflow::pressure_solver::multigrid_cpu::MultigridCPU;
use stormflow::pressure_solver::settings::MultigridSettings;

#[test]
fn cpu_pressure_solver_bench() {
    let grid = Grid::new(
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 1.0]),
        [96, 96, 96]
    );

    let boundary_conditions = PressureBoundaryConditions::new_from_up_direction(SpatialVector([0.0, 1.0, 0.0]));
    let settings = MultigridSettings::default();

    let mut cpu_solver = MultigridCPU::new(&grid, &boundary_conditions, settings);

    let n = grid.nr_interior_cells();
    let mut rhs = vec![0.0 as Float; n];
    for flat in 0..n {
        let [i, j, k] = grid.interior_indices_from_flat_index(flat);
        rhs[flat] = ((i + 1) as Float).sin() + ((j + 1) as Float).cos() * 0.5 - ((k + 1) as Float) * 0.01;
    }
    cpu_solver.rhs_at_levels[0].copy_from_slice(&rhs);

    // warm-up
    cpu_solver.solve();

    let start = Instant::now();
    for _ in 0..5 {
        cpu_solver.solve();
    }
    println!("[timing] avg solve() over 5 calls: {:?}", start.elapsed() / 5);
}
