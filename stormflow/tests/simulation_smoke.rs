use stormflow::simulation::builder::SimulationBuilder;

const SETTINGS: &str = r#"
{
    "grid": {
        "start_point": [-2.0, -1.0, -1.0],
        "end_point": [2.0, 1.0, 1.0],
        "cells_per_length_background": [4, 4, 4],
        "representative_length": 1.0
    },
    "wind_condition": {
        "direction_coming_from": 0.0,
        "velocity_variation": {"Constant": 5.0}
    },
    "linear_velocity": {"x": 0.0, "y": 0.0, "z": 0.0},
    "geometries": [
        {"Sphere": {"center": [0.0, 0.0, 0.0], "radius": 0.3}}
    ],
    "pressure_solver": {
        "Multigrid": {"nr_v_cycles": 1, "compute_platform": "CPU"}
    }
}
"#;

#[test]
fn simulation_runs_a_step_on_the_default_uniform_grid() {
    let builder = SimulationBuilder::from_json_str(SETTINGS).expect("settings should parse");

    let mut simulation = builder.build();
    simulation.initialize_after_build();

    let time_step = simulation.time_step_from_courant_number(0.5);
    assert!(time_step.is_finite() && time_step > 0.0);

    simulation.do_step(0.0, time_step);

    for velocity in &simulation.velocity_solver.velocity {
        for axis in 0..3 {
            assert!(velocity[axis].is_finite(), "velocity became non-finite");
        }
    }
}
