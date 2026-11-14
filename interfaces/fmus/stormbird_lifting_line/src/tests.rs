// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of the FMU logic, by calling the FMU functions directly on the struct

use super::*;

use std::path::Path;

use serde_json::{json, Value};

use stormbird::lifting_line::simulation_builder::{SimulationBuilder, SimulationSettings};
use stormbird::line_force_model::builder::LineForceModelBuilder;
use stormbird::line_force_model::builder::single_wing::WingBuilder;
use stormbird::section_models::SectionModel;
use stormbird::section_models::foil::Foil;

const TIME_STEP: f64 = 0.1;

/// Lifting line simulation with the given number of sails
fn simulation_builder(nr_sails: usize) -> SimulationBuilder {
    let mut line_force_model = LineForceModelBuilder::new(8);

    for i in 0..nr_sails {
        let x = 20.0 * i as f64;

        line_force_model.add_wing(WingBuilder {
            section_points: vec![
                SpatialVector::new(x, 0.0, 5.0),
                SpatialVector::new(x, 0.0, 35.0),
            ],
            chord_vectors: vec![SpatialVector::new(8.0, 0.0, 0.0); 2],
            section_model: SectionModel::SingleElementFoil(Foil::default()),
            ..Default::default()
        });
    }

    SimulationBuilder::new(line_force_model, SimulationSettings::default())
}

/// Parameters with a sail model without controller and wind environment, which are optional
fn parameters(nr_sails: usize) -> Value {
    json!({
        "sail_model": {
            "lifting_line_simulation": serde_json::to_value(simulation_builder(nr_sails)).unwrap()
        },
        "angles_in_degrees": true,
    })
}

/// Writes the content to a file in a temporary folder that is unique for each test
fn write_file(test_name: &str, content: &str) -> PathBuf {
    let folder = std::env::temp_dir().join("stormbird_lifting_line_fmu_tests").join(test_name);

    std::fs::create_dir_all(&folder).unwrap();

    let path = folder.join("parameters.json");

    std::fs::write(&path, content).unwrap();

    path
}

/// Creates the FMU with the parameters file at the given path, and exits the initialization mode
fn initialized_fmu(parameters_path: &Path) -> StormbirdLiftingLine {
    let mut fmu = StormbirdLiftingLine {
        parameters_path: parameters_path.to_string_lossy().to_string(),
        wind_velocity: 8.0,
        wind_direction_coming_from: 90.0,
        local_wing_angle_1: 10.0,
        local_wing_angle_2: 10.0,
        ..Default::default()
    };

    fmu.exit_initialization_mode();

    fmu
}

fn run_steps(fmu: &mut StormbirdLiftingLine, nr_steps: usize) {
    for i in 0..nr_steps {
        fmu.do_step(i as f64 * TIME_STEP, TIME_STEP);
    }
}

fn force_magnitude(fmu: &StormbirdLiftingLine) -> f64 {
    SpatialVector::new(fmu.force_x, fmu.force_y, fmu.force_z).length()
}

#[test]
fn valid_input_gives_forces_and_no_error() {
    let path = write_file("valid_input", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    assert_eq!(fmu.error_code, error_codes::OK);

    run_steps(&mut fmu, 5);

    assert_eq!(fmu.error_code, error_codes::OK);
    assert!(force_magnitude(&fmu) > 0.0);
    assert!(fmu.force_sail_2_y.abs() > 0.0);
    assert_eq!(fmu.force_sail_3_y, 0.0);
    assert!(fmu.velocity_measurement_1 > 0.0);
    assert!(fmu.angle_of_attack_measurement_1.abs() > 1.0, "The angle should be in degrees");
}

#[test]
fn missing_parameters_file_gives_io_error() {
    let mut fmu = initialized_fmu(Path::new("this_file_does_not_exist.json"));

    assert_eq!(fmu.error_code, 1);

    run_steps(&mut fmu, 2);

    assert_eq!(fmu.error_code, 1);
    assert_eq!(force_magnitude(&fmu), 0.0);
}

#[test]
fn invalid_json_gives_parse_error() {
    let path = write_file("invalid_json", "{ this is not json");

    assert_eq!(initialized_fmu(&path).error_code, 2);

    let mut unknown_field = parameters(2);
    unknown_field["unknown_field"] = json!(1.0);

    let path = write_file("unknown_field", &unknown_field.to_string());

    assert_eq!(initialized_fmu(&path).error_code, 2);
}

#[test]
fn invalid_model_gives_invalid_input_error() {
    let mut invalid = parameters(2);

    invalid["sail_model"]["lifting_line_simulation"]["line_force_model"]["wing_builders"][0]
        ["section_points"] = json!([{"x": 0.0, "y": 0.0, "z": 0.0}]);

    let path = write_file("invalid_model", &invalid.to_string());

    assert_eq!(initialized_fmu(&path).error_code, 3);
}

#[test]
fn too_many_sails_gives_an_error() {
    let path = write_file("too_many_sails", &parameters(MAX_NR_SAILS + 1).to_string());

    assert_eq!(initialized_fmu(&path).error_code, error_codes::TOO_MANY_SAILS);
}

#[test]
fn step_without_initialization_gives_an_error() {
    let mut fmu = StormbirdLiftingLine::default();

    fmu.do_step(0.0, TIME_STEP);

    assert_eq!(fmu.error_code, error_codes::MODEL_NOT_INITIALIZED);
}

#[test]
fn errors_during_simulation_are_latched() {
    let path = write_file("latched_error", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    run_steps(&mut fmu, 2);

    assert!(force_magnitude(&fmu) > 0.0);

    fmu.wind_velocity = f64::NAN;

    run_steps(&mut fmu, 1);

    assert_eq!(fmu.error_code, error_codes::NON_FINITE_INPUT);
    assert_eq!(force_magnitude(&fmu), 0.0);
    assert_eq!(fmu.velocity_measurement_1, 0.0);

    let error_code = fmu.error_code;

    // The model is not simulated again, even if the input is valid
    fmu.wind_velocity = 8.0;

    run_steps(&mut fmu, 2);

    assert_eq!(fmu.error_code, error_code);
    assert_eq!(force_magnitude(&fmu), 0.0);
}

#[test]
fn controller_sets_the_wing_angles() {
    let mut with_controller = parameters(1);

    with_controller["sail_model"]["controller"] = json!({
        "set_points": [{
            "apparent_wind_directions_data": [-3.2, 3.2],
            "angle_of_attack_data": [0.2, 0.2]
        }]
    });

    let path = write_file("controller", &with_controller.to_string());

    let mut fmu = initialized_fmu(&path);

    run_steps(&mut fmu, 3);

    assert_eq!(fmu.error_code, error_codes::OK);

    let local_wing_angle = fmu.sail_model.as_ref().unwrap()
        .lifting_line_simulation.line_force_model.local_wing_angles[0];

    assert!((local_wing_angle - 10_f64.to_radians()).abs() > 1e-3);
}

#[test]
fn panics_during_simulation_are_caught() {
    let path = write_file("panic", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    // Corrupt the internal state of the model, so that the next step panics on an index out of
    // bounds
    fmu.sail_model.as_mut().unwrap()
        .lifting_line_simulation.line_force_model.local_wing_angles.clear();

    run_steps(&mut fmu, 1);

    assert_eq!(fmu.error_code, error_codes::PANIC_DURING_TIME_STEP);
    assert!(fmu.sail_model.is_none());
}

fn angular_velocity(fmu: &StormbirdLiftingLine) -> SpatialVector {
    fmu.sail_model.as_ref().unwrap()
        .lifting_line_simulation.line_force_model.rigid_body_motion.velocity_angular
}

#[test]
/// In calm wind, the sails should still feel the apparent wind from the motion of the ship
fn calm_wind_with_moving_ship_gives_forces() {
    let path = write_file("calm_wind", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    fmu.wind_velocity = 0.0;

    for i in 0..5 {
        let time = i as f64 * TIME_STEP;

        fmu.translation_x = -6.0 * time;
        fmu.do_step(time, TIME_STEP);
    }

    assert_eq!(fmu.error_code, error_codes::OK);
    assert!((fmu.calculated_motion_velocity_linear_x + 6.0).abs() < 1e-9);
    assert!(force_magnitude(&fmu) > 0.0);
}

#[test]
/// A non-zero position in the first time step should not give a velocity, as there is no previous
/// position
fn non_zero_initial_position_gives_no_velocity_spike() {
    let path = write_file("initial_position", &parameters(2).to_string());

    let mut far_away = initialized_fmu(&path);
    let mut at_origin = initialized_fmu(&path);

    for fmu in [&mut far_away, &mut at_origin] {
        fmu.rotation_z = 90.0;
    }

    far_away.translation_x = 1000.0;
    far_away.translation_y = -500.0;

    far_away.do_step(0.0, TIME_STEP);
    at_origin.do_step(0.0, TIME_STEP);

    assert_eq!(far_away.calculated_motion_velocity_linear_x, 0.0);
    assert_eq!(far_away.calculated_motion_velocity_angular_z, 0.0);

    // The wind only varies with height, so the forces should be the same
    let difference = SpatialVector::new(
        far_away.force_x - at_origin.force_x,
        far_away.force_y - at_origin.force_y,
        far_away.force_z - at_origin.force_z,
    );

    assert!(force_magnitude(&at_origin) > 0.0);
    assert!(difference.length() < 1e-6 * force_magnitude(&at_origin));
}

#[test]
/// The motion should be tracked while waiting for the simulation to start, so that the velocity is
/// correct in the first simulated time step
fn motion_is_tracked_while_waiting() {
    let mut waiting = parameters(2);
    waiting["number_of_iterations_before_building_model"] = json!(3);

    let path = write_file("waiting", &waiting.to_string());

    let mut fmu = initialized_fmu(&path);

    for i in 0..4 {
        let time = i as f64 * TIME_STEP;

        fmu.translation_x = 100.0 - 6.0 * time;
        fmu.do_step(time, TIME_STEP);
    }

    assert_eq!(fmu.error_code, error_codes::OK);
    assert_eq!(fmu.model_steps_completed, 1);
    assert!((fmu.calculated_motion_velocity_linear_x + 6.0).abs() < 1e-9);
}

#[test]
/// With a heading of 90 degrees, the roll motion is around the x-axis of the ship, which points
/// along the global y-axis
fn roll_with_heading_gives_angular_velocity_around_the_ship_axis() {
    let path = write_file("roll_with_heading", &parameters(2).to_string());

    let mut fmu = initialized_fmu(&path);

    let roll_rate_degrees = 2.0;

    fmu.rotation_z = 90.0;

    let mut previous_ctrl_points: Vec<SpatialVector> = Vec::new();

    for i in 0..4 {
        let time = i as f64 * TIME_STEP;

        fmu.rotation_x = roll_rate_degrees * time;

        previous_ctrl_points = fmu.sail_model.as_ref().unwrap()
            .lifting_line_simulation.line_force_model.ctrl_points_global.clone();

        fmu.do_step(time, TIME_STEP);
    }

    assert_eq!(fmu.error_code, error_codes::OK);

    let expected = SpatialVector::new(0.0, roll_rate_degrees.to_radians(), 0.0);

    assert!(
        (angular_velocity(&fmu) - expected).length() < 1e-9,
        "angular velocity: {:?}", angular_velocity(&fmu)
    );

    // The velocity of the control points should match the change in their position
    let line_force_model = &fmu.sail_model.as_ref().unwrap().lifting_line_simulation.line_force_model;

    for (point, previous) in line_force_model.ctrl_points_global.iter().zip(&previous_ctrl_points) {
        let fd_velocity = (*point - *previous) / TIME_STEP;
        let velocity = line_force_model.rigid_body_motion.velocity_at_point(0.5 * (*point + *previous));

        assert!(
            (velocity - fd_velocity).length() < 1e-6 * (1.0 + fd_velocity.length()),
            "velocity: {:?}, finite difference: {:?}", velocity, fd_velocity
        );
    }
}

#[test]
/// An angular velocity input in the body fixed frame should be converted to the global frame
fn body_fixed_angular_velocity_input_is_converted_to_global() {
    let mut body_fixed = parameters(2);
    body_fixed["use_motion_velocity"] = json!(true);
    body_fixed["motion_velocity_in_body_fixed_frame"] = json!(true);

    let path = write_file("body_fixed_angular_velocity", &body_fixed.to_string());

    let mut fmu = initialized_fmu(&path);

    fmu.rotation_z = 90.0;
    fmu.motion_velocity_angular_x = 0.1_f64.to_degrees();

    run_steps(&mut fmu, 2);

    assert_eq!(fmu.error_code, error_codes::OK);

    let expected = SpatialVector::new(0.0, 0.1, 0.0);

    assert!(
        (angular_velocity(&fmu) - expected).length() < 1e-9,
        "angular velocity: {:?}", angular_velocity(&fmu)
    );
}

fn steady_state_parameters(nr_sails: usize) -> Value {
    let mut parameters = parameters(nr_sails);

    parameters["steady_state"] = json!(true);

    parameters
}

fn controller_json() -> Value {
    json!({
        "set_points": [{
            "apparent_wind_directions_data": [-3.2, 3.2],
            "angle_of_attack_data": [0.2, 0.2]
        }, {
            "apparent_wind_directions_data": [-3.2, 3.2],
            "angle_of_attack_data": [0.2, 0.2]
        }]
    })
}

fn force(fmu: &StormbirdLiftingLine) -> SpatialVector {
    SpatialVector::new(fmu.force_x, fmu.force_y, fmu.force_z)
}

#[test]
/// The steady state mode should give the same result as the steady state method of the complete
/// sail model, with the ship velocity from the linear motion velocity input
fn steady_state_mode_matches_the_complete_sail_model() {
    let parameters = steady_state_parameters(2);

    let path = write_file("steady_state_mode", &parameters.to_string());

    let mut fmu = initialized_fmu(&path);

    let ship_speed = 6.0;

    fmu.motion_velocity_linear_x = -ship_speed;
    fmu.controller_loading = 1.0;

    run_steps(&mut fmu, 1);

    assert_eq!(fmu.error_code, error_codes::OK);

    let mut model: CompleteSailModel = serde_json::from_value::<
        stormbird::lifting_line::complete_sail_model::builder::CompleteSailModelBuilder
    >(parameters["sail_model"].clone()).unwrap().build().unwrap();

    model.set_local_wing_angles(&[10_f64.to_radians(), 10_f64.to_radians()]);

    let wind_condition = WindCondition {
        direction_coming_from: 90_f64.to_radians(),
        velocity_variation: VelocityVariation::PowerModel(PowerModel {
            reference_velocity: 8.0,
            reference_height: PowerModel::default_reference_height(),
            power_factor: PowerModel::default_power_factor(),
        }),
        parallel_gust: None,
        perpendicular_gust: None,
        vertical_gust: None,
    };

    let expected = model.simulate_steady_state_condition(&wind_condition, ship_speed, 1.0)
        .integrated_forces_sum();

    assert!(
        (force(&fmu) - expected).length() < 1e-6 * expected.length(),
        "FMU: {:?}, complete sail model: {:?}", force(&fmu), expected
    );
}

#[test]
/// In the steady state mode, the result should not depend on the time step or on earlier steps,
/// and the motion of the sails should not give any velocity
fn steady_state_mode_is_independent_of_time() {
    let mut parameters = steady_state_parameters(2);
    parameters["sail_model"]["controller"] = controller_json();

    let path = write_file("steady_state_time", &parameters.to_string());

    let mut fmu = initialized_fmu(&path);

    fmu.motion_velocity_linear_x = -6.0;
    fmu.controller_loading = 1.0;

    fmu.do_step(0.0, TIME_STEP);

    let first = force(&fmu);

    fmu.rotation_x = 5.0;
    fmu.do_step(TIME_STEP, 10.0 * TIME_STEP);

    fmu.rotation_x = 0.0;
    fmu.do_step(11.0 * TIME_STEP, 0.01);

    assert_eq!(fmu.error_code, error_codes::OK);
    assert_eq!(fmu.calculated_motion_velocity_angular_x, 0.0);
    assert!((force(&fmu) - first).length() < 1e-6 * first.length());
}

#[test]
fn steady_state_mode_with_optimized_loading() {
    let mut parameters = steady_state_parameters(2);
    parameters["sail_model"]["controller"] = controller_json();
    parameters["steady_state_optimize_loading"] = json!(true);

    let path = write_file("steady_state_optimize", &parameters.to_string());

    let mut fmu = initialized_fmu(&path);

    fmu.motion_velocity_linear_x = -6.0;
    fmu.controller_loading = 1.0;

    run_steps(&mut fmu, 1);

    assert_eq!(fmu.error_code, error_codes::OK);
    assert!(force_magnitude(&fmu) > 0.0);
}

#[test]
fn inconsistent_steady_state_settings_give_errors() {
    let mut dynamic = steady_state_parameters(2);
    dynamic["sail_model"]["lifting_line_simulation"]["simulation_settings"] = json!({"Dynamic": {}});

    let path = write_file("steady_state_dynamic", &dynamic.to_string());

    assert_eq!(initialized_fmu(&path).error_code, 3);

    let mut optimize_only = parameters(2);
    optimize_only["steady_state_optimize_loading"] = json!(true);

    let path = write_file("optimize_without_steady_state", &optimize_only.to_string());

    assert_eq!(initialized_fmu(&path).error_code, 3);
}
