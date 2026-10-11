use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::grid::{Grid, INTERIOR_OFFSET};
use stormflow::simulation::Simulation;

const NR_STEPS: usize = 5;

/// The rotor sail example with end disks (actuator line + slip geometries), on a coarser grid, with
/// an extra no-slip sphere, so that every part of the velocity solver is exercised. The coarsest
/// multigrid level is solved with Jacobi iterations, to keep the test fast. `boundary_case`
/// optionally changes the boundary conditions, see `BoundaryCase`.
fn setup_string(velocity_platform: &str, pressure_platform: &str, boundary_case: BoundaryCase) -> String {
    let example_path = concat!(
        env!("CARGO_MANIFEST_DIR"), "/examples/rotor_sail/rotor_sail_single_with_end_disks.json"
    );

    let mut setup: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(example_path).unwrap()
    ).unwrap();

    setup["grid"]["cells_per_representative_length"] = serde_json::json!([6, 6, 6]);
    setup["velocity_solver"] = serde_json::json!({
        "compute_platform": velocity_platform,
        "no_slip_wall_treatment": "DataImmersion",
        "slip_sharp_edges": {"enabled": false}
    });
    setup["pressure_solver"]["Multigrid"]["compute_platform"] = pressure_platform.into();
    setup["pressure_solver"]["Multigrid"]["compute_residual_after_solve"] = false.into();
    setup["pressure_solver"]["Multigrid"]["coarsest_level_solver"] = "Jacobi".into();
    setup["actuator_line"]["write_iterations_full_result"] = 1_000_000.into();
    setup["geometries"] = serde_json::json!([
        {"Sphere": {"center": {"x": 20.0, "y": 5.0, "z": 10.0}, "radius": 4.0}}
    ]);

    if let BoundaryCase::WallModel = boundary_case {
        setup["velocity_solver"]["no_slip_wall_treatment"] = "WallModel".into();
        setup["effective_viscosity"] = 1.5e-5.into();

        // A cuboid standing on the ground, which exercises the zeroing of the faces where the
        // mirror image is outside the domain
        setup["geometries"].as_array_mut().unwrap().push(serde_json::json!(
            {"Cuboid": {"center": {"x": 30.0, "y": 0.0, "z": 8.0}, "half_extents": {"x": 4.0, "y": 6.0, "z": 8.0}}}
        ));
    }

    if let BoundaryCase::VelocityLimiter = boundary_case {
        // Low enough to clip the velocity close to the rotor sail
        setup["velocity_solver"]["max_velocity_factor"] = 1.1.into();
    }

    if let BoundaryCase::SlipCuboidOnGroundWithSharpEdges = boundary_case {
        setup["velocity_solver"]["slip_sharp_edges"] = serde_json::json!({"enabled": true});
    }

    if let BoundaryCase::SlipCuboidOnGround | BoundaryCase::SlipCuboidOnGroundWithSharpEdges = boundary_case {
        setup["slip_geometries"].as_array_mut().unwrap().push(serde_json::json!(
            {"Cuboid": {"center": {"x": 30.0, "y": 0.0, "z": 8.0}, "half_extents": {"x": 4.0, "y": 6.0, "z": 8.0}}}
        ));
    }

    if let BoundaryCase::DownwardUpDirectionAndSideWalls = boundary_case {
        setup["wind_environment"] = serde_json::json!({
            "up_direction": {"x": 0.0, "y": 0.0, "z": -1.0},
            "wind_rotation_axis": {"x": 0.0, "y": 0.0, "z": -1.0}
        });
        setup["wind_condition"]["direction_coming_from"] = 0.3.into();
        setup["slip_wall_boundary_override"] = serde_json::json!([[false, false], [true, true], [false, false]]);
    }

    setup.to_string()
}

#[derive(Debug, Clone, Copy)]
enum BoundaryCase {
    /// The boundary conditions of the example: the ground at the start of the z-axis
    Default,
    /// The up direction along the negative z-axis, which puts the ground at the end of the z-axis,
    /// and slip walls on both y faces, with an oblique wind, so that slip walls at both the start
    /// and the end of an axis are exercised.
    DownwardUpDirectionAndSideWalls,
    /// The boundary conditions of the example, with an extra slip cuboid standing on the ground
    SlipCuboidOnGround,
    /// The same as `SlipCuboidOnGround`, with the data immersion close to the sharp edges of the
    /// slip geometries
    SlipCuboidOnGroundWithSharpEdges,
    /// The boundary conditions of the example, with the velocity limiter
    VelocityLimiter,
    /// The boundary conditions of the example, with the wall model instead of the data immersion,
    /// for the no-slip sphere and a cuboid standing on the ground
    WallModel,
}

struct SimulationOutput {
    grid: Grid,
    time_steps: Vec<Float>,
    velocity: Vec<SpatialVector>,
    body_force: Vec<SpatialVector>,
    pressure: Vec<Float>,
    /// The number of velocity values clipped by the velocity limiter in each time step
    nr_limited_values: Vec<usize>,
}

fn run(velocity_platform: &str, pressure_platform: &str, boundary_case: BoundaryCase) -> SimulationOutput {
    let mut sim = Simulation::new_from_string(&setup_string(velocity_platform, pressure_platform, boundary_case)).unwrap();

    let mut time_steps = Vec::with_capacity(NR_STEPS);
    let mut nr_limited_values = Vec::with_capacity(NR_STEPS);
    let mut time = 0.0;

    for _ in 0..NR_STEPS {
        let time_step = sim.time_step_from_courant_number(0.75);

        sim.do_step(time, time_step);

        time_steps.push(time_step);
        nr_limited_values.push(sim.nr_limited_velocity_values());
        time += time_step;
    }

    SimulationOutput {
        time_steps,
        velocity: sim.velocity_solver.velocity_host().to_vec(),
        body_force: sim.velocity_solver.body_force_host().to_vec(),
        pressure: sim.pressure_solver.pressure_host().to_vec(),
        nr_limited_values,
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

fn assert_gpu_velocity_matches_cpu(pressure_platform: &str, boundary_case: BoundaryCase) {
    let reference = run("CPU", "CPU", boundary_case);
    let gpu = run("GPU", pressure_platform, boundary_case);

    let grid = &reference.grid;

    for (cpu_time_step, gpu_time_step) in reference.time_steps.iter().zip(&gpu.time_steps) {
        let relative_diff = (cpu_time_step - gpu_time_step).abs() / cpu_time_step;

        assert!(relative_diff < 1e-5, "Time steps differ: {cpu_time_step} vs {gpu_time_step}");
    }

    println!("Limited values per step: CPU {:?}, GPU {:?}", reference.nr_limited_values, gpu.nr_limited_values);

    if let BoundaryCase::VelocityLimiter = boundary_case {
        let max_velocity = reference.velocity.iter()
            .flat_map(|v| [v[0].abs(), v[1].abs(), v[2].abs()])
            .fold(0.0, Float::max);

        let inlet_velocity = setup_inlet_velocity_magnitude();

        assert!(reference.nr_limited_values.iter().sum::<usize>() > 0, "The limiter was never active");
        assert!(max_velocity <= 1.1 * inlet_velocity * (1.0 + 1e-5), "{max_velocity} exceeds the limit");

        // The counts can differ slightly, from values that are rounded to either side of the limit
        for (cpu, gpu) in reference.nr_limited_values.iter().zip(&gpu.nr_limited_values) {
            let difference = (*cpu as Float - *gpu as Float).abs();

            assert!(difference <= 0.01 * (*cpu as Float) + 2.0, "Limited values differ: {cpu} vs {gpu}");
        }
    } else {
        assert!(reference.nr_limited_values.iter().all(|&n| n == 0));
        assert!(gpu.nr_limited_values.iter().all(|&n| n == 0));
    }

    let velocity_diff = max_interior_diff(grid, &reference.velocity, &gpu.velocity);
    let body_force_diff = max_interior_diff(grid, &reference.body_force, &gpu.body_force);

    let mut pressure_diff: Float = 0.0;
    let mut pressure_max: Float = 0.0;

    for (flat_index, (a, b)) in reference.pressure.iter().zip(&gpu.pressure).enumerate() {
        if is_interior(grid, flat_index) {
            pressure_diff = pressure_diff.max((a - b).abs());
            pressure_max = pressure_max.max(a.abs());
        }
    }

    let velocity_max = reference.velocity.iter().map(|v| v.length()).fold(0.0, Float::max);
    let body_force_max = reference.body_force.iter().map(|v| v.length()).fold(0.0, Float::max);

    println!(
        "max |CPU - GPU| in interior cells, relative to the field magnitude: velocity = {:e}, body force = {:e}, pressure = {:e}",
        velocity_diff / velocity_max, body_force_diff / body_force_max, pressure_diff / pressure_max
    );

    // The differences are rounding errors, accumulated over the time steps
    assert!(velocity_diff < 1e-5 * velocity_max, "GPU velocity diverges from the CPU velocity by {velocity_diff}");
    assert!(body_force_diff < 1e-5 * body_force_max, "GPU body force diverges from the CPU body force by {body_force_diff}");
    assert!(pressure_diff < 1e-4 * pressure_max, "Pressure with the GPU velocity solver diverges by {pressure_diff}");
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_cpu_pressure_solver() {
    assert_gpu_velocity_matches_cpu("CPU", BoundaryCase::Default);
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_gpu_pressure_solver() {
    assert_gpu_velocity_matches_cpu("GPU", BoundaryCase::Default);
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_downward_up_direction_and_side_walls() {
    assert_gpu_velocity_matches_cpu("GPU", BoundaryCase::DownwardUpDirectionAndSideWalls);
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_wall_model() {
    assert_gpu_velocity_matches_cpu("GPU", BoundaryCase::WallModel);
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_slip_cuboid_on_ground() {
    assert_gpu_velocity_matches_cpu("GPU", BoundaryCase::SlipCuboidOnGround);
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_slip_sharp_edges() {
    assert_gpu_velocity_matches_cpu("GPU", BoundaryCase::SlipCuboidOnGroundWithSharpEdges);
}

/// The largest inlet velocity magnitude of the example, which the velocity limit is relative to
fn setup_inlet_velocity_magnitude() -> Float {
    let builder = stormflow::simulation::builder::SimulationBuilder::new_from_string(
        &setup_string("CPU", "CPU", BoundaryCase::VelocityLimiter)
    ).unwrap();

    let grid = builder.grid.build_from_line_force_model_builder(
        &builder.actuator_line.as_ref().unwrap().line_force_model
    );

    let boundary_conditions = stormflow::velocity_solver::boundary_condisitions::VelocityBoundaryConditions::new(
        &builder.wind_environment,
        &builder.wind_condition,
        builder.linear_velocity,
        builder.slip_wall_boundary_override,
        &grid
    );

    builder.velocity_solver.velocity_limit(&boundary_conditions).unwrap() / 1.1
}

#[test]
fn gpu_velocity_solver_matches_cpu_with_velocity_limiter() {
    assert_gpu_velocity_matches_cpu("GPU", BoundaryCase::VelocityLimiter);
}
