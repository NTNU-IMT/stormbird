use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use stormflow::grid::{Grid, INTERIOR_OFFSET};
use stormflow::simulation::Simulation;

const NR_STEPS: usize = 5;

/// The rotor sail example with end disks (actuator line + slip geometries), on a coarser grid, with
/// an extra no-slip sphere and the realizable k-epsilon model, so that every part of the
/// turbulence solver is exercised: the inlet/outlet conditions, the mirror corrections in both slip
/// and no-slip geometries, the wall functions and the damping of the eddy viscosity.
fn setup_string(platform: &str, inlet: serde_json::Value, wall_treatment: &str) -> String {
    let example_path = concat!(
        env!("CARGO_MANIFEST_DIR"), "/examples/rotor_sail/rotor_sail_single_with_end_disks.json"
    );

    let mut setup: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(example_path).unwrap()
    ).unwrap();

    setup["grid"]["cells_per_representative_length"] = serde_json::json!([6, 6, 6]);
    setup["velocity_solver_compute_platform"] = platform.into();
    setup["pressure_solver"]["Multigrid"]["compute_platform"] = platform.into();
    setup["pressure_solver"]["Multigrid"]["compute_residual_after_solve"] = false.into();
    setup["pressure_solver"]["Multigrid"]["coarsest_level_solver"] = "Jacobi".into();
    setup["actuator_line"]["write_iterations_full_result"] = 1_000_000.into();
    setup["effective_viscosity"] = 1.5e-5.into();
    setup["geometries"] = serde_json::json!([
        {"Sphere": {"center": {"x": 20.0, "y": 5.0, "z": 10.0}, "radius": 4.0}}
    ]);
    setup["no_slip_wall_treatment"] = wall_treatment.into();
    setup["turbulence"] = serde_json::json!({
        "model": {"RealizableKEpsilon": {}},
        "inlet": inlet,
    });

    setup.to_string()
}

struct SimulationOutput {
    grid: Grid,
    time_steps: Vec<Float>,
    velocity: Vec<SpatialVector>,
    eddy_viscosity: Vec<Float>,
    turbulence_fields: Vec<Float>,
}

fn run(platform: &str, inlet: serde_json::Value, wall_treatment: &str) -> SimulationOutput {
    let mut sim = Simulation::new_from_string(&setup_string(platform, inlet, wall_treatment)).unwrap();

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
        eddy_viscosity: sim.velocity_solver.eddy_viscosity_host().unwrap().to_vec(),
        turbulence_fields: sim.turbulence_solver.as_ref().unwrap().fields_host().to_vec(),
        grid: sim.grid,
    }
}

/// Number of interior cells next to the domain boundary that are excluded from the comparison
const BOUNDARY_MARGIN: usize = 2;

/// Whether the cell is in the interior, and at least `BOUNDARY_MARGIN` cells away from the
/// boundary of the domain.
fn is_compared(grid: &Grid, flat_index: usize) -> bool {
    let indices = grid.extended_indices_from_flat_index(flat_index);

    (0..3).all(|axis| {
        indices[axis] >= INTERIOR_OFFSET + BOUNDARY_MARGIN &&
        indices[axis] + BOUNDARY_MARGIN < INTERIOR_OFFSET + grid.interior_shape[axis]
    })
}

/// Max absolute difference over the compared cells, relative to the max absolute value. The ghost
/// cells are excluded for the same reason as in the velocity solver test: the inlet/outlet
/// condition switches discontinuously where the flow normal to the boundary is close to zero, so
/// rounding differences can legitimately pick different ghost values. With a turbulence model,
/// this also affects the interior cells next to the boundary, as the eddy viscosity there depends
/// on the velocity gradient computed from the ghost cells, and is very sensitive to small velocity
/// gradients where k / epsilon is large.
fn max_relative_interior_diff(grid: &Grid, a: &[Float], b: &[Float]) -> Float {
    let n = grid.nr_extended_cells();

    let mut max_diff: Float = 0.0;
    let mut max_value: Float = 0.0;

    for (index, (a, b)) in a.iter().zip(b).enumerate() {
        assert!(a.is_finite() && b.is_finite());

        if is_compared(grid, index % n) {
            max_diff = max_diff.max((a - b).abs());
            max_value = max_value.max(a.abs());
        }
    }

    max_diff / max_value
}

/// Flattens the vectors component-major, so that the cell of index `i` is `i % vectors.len()`, as
/// for the other fields
fn flatten(vectors: &[SpatialVector]) -> Vec<Float> {
    (0..3).flat_map(|component| vectors.iter().map(move |v| v[component])).collect()
}

fn default_inlet() -> serde_json::Value {
    serde_json::json!({
        "IntensityAndLengthScale": {"turbulence_intensity": 0.05, "length_scale": 2.0}
    })
}

#[test]
fn cpu_turbulence_solver_gives_finite_positive_fields() {
    for wall_treatment in ["DataImmersion", "WallModel"] {
        let output = run("CPU", default_inlet(), wall_treatment);

        assert!(output.turbulence_fields.iter().all(|value| value.is_finite() && *value > 0.0));
        assert!(output.eddy_viscosity.iter().all(|value| value.is_finite() && *value >= 0.0));

        let max_eddy_viscosity = output.eddy_viscosity.iter().cloned().fold(0.0, Float::max);

        println!("{wall_treatment}: max eddy viscosity: {max_eddy_viscosity}");

        assert!(max_eddy_viscosity > 0.0);
    }
}

fn assert_gpu_matches_cpu(inlet: serde_json::Value, wall_treatment: &str) {
    let reference = run("CPU", inlet.clone(), wall_treatment);
    let gpu = run("GPU", inlet, wall_treatment);

    let grid = &reference.grid;

    for (cpu_time_step, gpu_time_step) in reference.time_steps.iter().zip(&gpu.time_steps) {
        let relative_diff = (cpu_time_step - gpu_time_step).abs() / cpu_time_step;

        assert!(relative_diff < 1e-5, "Time steps differ: {cpu_time_step} vs {gpu_time_step}");
    }

    let velocity_diff = max_relative_interior_diff(
        grid, &flatten(&reference.velocity), &flatten(&gpu.velocity)
    );
    let eddy_viscosity_diff = max_relative_interior_diff(grid, &reference.eddy_viscosity, &gpu.eddy_viscosity);
    let fields_diff = max_relative_interior_diff(grid, &reference.turbulence_fields, &gpu.turbulence_fields);

    println!(
        "max |CPU - GPU| in interior cells, relative to the field magnitude: velocity = {:e}, \
         eddy viscosity = {:e}, turbulence fields = {:e}",
        velocity_diff, eddy_viscosity_diff, fields_diff
    );

    // The differences are rounding errors, accumulated over the time steps. The eddy viscosity gets
    // a looser tolerance, as the realizable C_mu is very sensitive to small velocity gradients in
    // the free stream, where k / epsilon is large.
    assert!(velocity_diff < 1e-4, "GPU velocity diverges from the CPU velocity by {velocity_diff}");
    assert!(eddy_viscosity_diff < 2e-2, "GPU eddy viscosity diverges from the CPU version by {eddy_viscosity_diff}");
    assert!(fields_diff < 1e-3, "GPU turbulence fields diverge from the CPU version by {fields_diff}");
}

#[test]
fn gpu_turbulence_solver_matches_cpu() {
    assert_gpu_matches_cpu(default_inlet(), "DataImmersion");
}

#[test]
fn gpu_turbulence_solver_matches_cpu_with_wall_model() {
    assert_gpu_matches_cpu(default_inlet(), "WallModel");
}

#[test]
fn gpu_turbulence_solver_matches_cpu_with_atmospheric_boundary_layer_inlet() {
    assert_gpu_matches_cpu(serde_json::json!({
        "AtmosphericBoundaryLayer": {"friction_velocity": 0.5, "roughness_length": 0.01}
    }), "DataImmersion");
}


