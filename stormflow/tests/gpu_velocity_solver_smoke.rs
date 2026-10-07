use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::grid::{Grid, INTERIOR_OFFSET};
use stormflow::simulation::Simulation;

const NR_STEPS: usize = 5;

/// The rotor sail example with end disks (actuator line + slip geometries), on a coarser grid, with
/// an extra no-slip sphere, so that every part of the velocity solver is exercised. The coarsest
/// multigrid level is solved with Jacobi iterations, to keep the test fast.
fn setup_string(velocity_platform: &str, pressure_platform: &str) -> String {
    let example_path = concat!(
        env!("CARGO_MANIFEST_DIR"), "/examples/rotor_sail/rotor_sail_single_with_end_disks.json"
    );

    let mut setup: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(example_path).unwrap()
    ).unwrap();

    setup["grid"]["cells_per_representative_length"] = serde_json::json!([6, 6, 6]);
    setup["velocity_solver_compute_platform"] = velocity_platform.into();
    setup["pressure_solver"]["Multigrid"]["compute_platform"] = pressure_platform.into();
    setup["pressure_solver"]["Multigrid"]["compute_residual_after_solve"] = false.into();
    setup["pressure_solver"]["Multigrid"]["coarsest_level_solver"] = "Jacobi".into();
    setup["actuator_line"]["write_iterations_full_result"] = 1_000_000.into();
    setup["geometries"] = serde_json::json!([
        {"Sphere": {"center": {"x": 20.0, "y": 5.0, "z": 10.0}, "radius": 4.0}}
    ]);

    setup.to_string()
}

struct SimulationOutput {
    grid: Grid,
    time_steps: Vec<Float>,
    velocity: Vec<SpatialVector>,
    body_force: Vec<SpatialVector>,
    pressure: Vec<Float>,
}

fn run(velocity_platform: &str, pressure_platform: &str) -> SimulationOutput {
    let mut sim = Simulation::new_from_string(&setup_string(velocity_platform, pressure_platform)).unwrap();

    let mut time_steps = Vec::with_capacity(NR_STEPS);
    let mut time = 0.0;

    for _ in 0..NR_STEPS {
        let time_step = sim.time_step_from_courant_number(0.75);

        sim.do_step(time, time_step);

        time_steps.push(time_step);
        time += time_step;
    }

    SimulationOutput {
        time_steps,
        velocity: sim.velocity_solver.velocity_host().to_vec(),
        body_force: sim.velocity_solver.body_force_host().to_vec(),
        pressure: sim.pressure_solver.pressure_host().to_vec(),
        grid: sim.grid,
    }
}

fn is_interior(grid: &Grid, flat_index: usize) -> bool {
    let indices = grid.extended_indices_from_flat_index(flat_index);

    (0..3).all(|axis| {
        indices[axis] >= INTERIOR_OFFSET && indices[axis] < INTERIOR_OFFSET + grid.interior_shape[axis]
    })
}

/// Max absolute difference over the interior cells. The ghost cells are excluded, as the
/// inlet/outlet boundary condition switches discontinuously where the flow normal to the boundary
/// is close to zero, so rounding differences can legitimately pick different ghost values.
fn max_interior_diff(grid: &Grid, a: &[SpatialVector], b: &[SpatialVector]) -> Float {
    let mut max_diff: Float = 0.0;

    for (flat_index, (a, b)) in a.iter().zip(b).enumerate() {
        assert!(a[0].is_finite() && a[1].is_finite() && a[2].is_finite());

        if is_interior(grid, flat_index) {
            for component in 0..3 {
                max_diff = max_diff.max((a[component] - b[component]).abs());
            }
        }
    }

    max_diff
}

fn assert_gpu_velocity_matches_cpu(pressure_platform: &str) {
    let reference = run("CPU", "CPU");
    let gpu = run("GPU", pressure_platform);

    let grid = &reference.grid;

    for (cpu_time_step, gpu_time_step) in reference.time_steps.iter().zip(&gpu.time_steps) {
        let relative_diff = (cpu_time_step - gpu_time_step).abs() / cpu_time_step;

        assert!(relative_diff < 1e-5, "Time steps differ: {cpu_time_step} vs {gpu_time_step}");
    }

    let velocity_diff = max_interior_diff(grid, &reference.velocity, &gpu.velocity);
    let body_force_diff = max_interior_diff(grid, &reference.body_force, &gpu.body_force);

    let mut pressure_diff: Float = 0.0;

    for (flat_index, (a, b)) in reference.pressure.iter().zip(&gpu.pressure).enumerate() {
        if is_interior(grid, flat_index) {
            pressure_diff = pressure_diff.max((a - b).abs());
        }
    }

    println!("max |CPU - GPU| in interior cells: velocity = {velocity_diff:e}, body force = {body_force_diff:e}, pressure = {pressure_diff:e}");

    // The velocity is around 10 m/s and the pressure around 100 Pa in this case, so these are
    // differences of at most 1e-5 relative to the magnitude of the fields.
    assert!(velocity_diff < 1e-4, "GPU velocity diverges from the CPU velocity by {velocity_diff}");
    assert!(body_force_diff < 1e-4, "GPU body force diverges from the CPU body force by {body_force_diff}");
    assert!(pressure_diff < 1e-3, "Pressure with the GPU velocity solver diverges by {pressure_diff}");
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_cpu_pressure_solver() {
    assert_gpu_velocity_matches_cpu("CPU");
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_gpu_pressure_solver() {
    assert_gpu_velocity_matches_cpu("GPU");
}
