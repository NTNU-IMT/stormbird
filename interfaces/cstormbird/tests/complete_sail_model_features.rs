//! Tests of the complete sail model functions that follow the features in Stormbird 0.13.0, and of
//! the accessors for the simulation results.

use std::ffi::{CStr, CString};

use cstormbird::error::{stormbird_clear_last_error, stormbird_last_error_message};
use cstormbird::lifting_line::complete_sail_model::*;
use cstormbird::results::*;
use cstormbird::wind::*;

const TOLERANCE: f64 = 1e-9;

const SHIP_VELOCITY: f64 = 6.0;

/// Two wing sails, with an optional controller given as a JSON object
fn setup_string(controller: Option<&str>) -> CString {
    let wing = |x: f64| format!(
        r#"{{
            "section_points": [{{"x": {x}, "y": 0.0, "z": 5.0}}, {{"x": {x}, "y": 0.0, "z": 35.0}}],
            "chord_vectors": [{{"x": 8.0, "y": 0.0, "z": 0.0}}, {{"x": 8.0, "y": 0.0, "z": 0.0}}],
            "section_model": {{"SingleElementFoil": {{}}}}
        }}"#
    );

    let controller = match controller {
        Some(controller) => format!(r#", "controller": {}"#, controller),
        None => String::new(),
    };

    CString::new(format!(
        r#"{{
            "lifting_line_simulation": {{
                "line_force_model": {{
                    "nr_sections": 8,
                    "wing_builders": [{}, {}]
                }}
            }}{}
        }}"#,
        wing(-20.0), wing(20.0), controller
    )).unwrap()
}

const CONTROLLER: &str = r#"{
    "set_points": [
        {"apparent_wind_directions_data": [-3.2, 3.2], "angle_of_attack_data": [0.2, 0.2]},
        {"apparent_wind_directions_data": [-3.2, 3.2], "angle_of_attack_data": [0.2, 0.2]}
    ]
}"#;

fn new_model(controller: Option<&str>) -> *mut CompleteSailModel {
    let model = complete_sail_model_new(setup_string(controller).as_ptr());

    assert!(!model.is_null(), "model could not be created: {:?}", last_error_message());

    model
}

fn last_error_message() -> Option<String> {
    let message = stormbird_last_error_message();

    if message.is_null() {
        return None;
    }

    Some(unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned())
}

fn beam_wind() -> *mut WindCondition {
    wind_condition_new_constant(std::f64::consts::FRAC_PI_2, 8.0)
}

fn forces_sum(result: *mut SimulationResult) -> [f64; 3] {
    let mut force = [0.0; 3];

    assert_eq!(simulation_result_integrated_forces_sum(result, force.as_mut_ptr()), 0);

    force
}

fn assert_close(a: [f64; 3], b: [f64; 3]) {
    let scale = 1.0 + a.iter().map(|v| v.abs()).fold(0.0, f64::max);

    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < TOLERANCE * scale, "{:?} != {:?}", a, b);
    }
}

#[test]
fn the_controller_is_optional() {
    let with_controller = new_model(Some(CONTROLLER));
    let without_controller = new_model(None);

    assert_eq!(complete_sail_model_has_controller(with_controller), 1);
    assert_eq!(complete_sail_model_has_controller(without_controller), 0);
    assert_eq!(complete_sail_model_has_controller(std::ptr::null()), -1);

    complete_sail_model_drop(with_controller);
    complete_sail_model_drop(without_controller);
}

#[test]
fn ship_velocity_as_vector_gives_the_same_results_as_scalar() {
    let model = new_model(Some(CONTROLLER));
    let wind = beam_wind();

    let mut ship_velocity_vector = [0.0; 3];

    assert_eq!(
        complete_sail_model_ship_velocity_vector(model, SHIP_VELOCITY, ship_velocity_vector.as_mut_ptr()),
        0
    );

    assert_close(ship_velocity_vector, [-SHIP_VELOCITY, 0.0, 0.0]);

    // Steady state
    let scalar = complete_sail_model_simulate_steady_state_condition(model, wind, SHIP_VELOCITY, 1.0);
    let vector = complete_sail_model_simulate_steady_state_condition_with_ship_velocity_vector(
        model, wind, ship_velocity_vector.as_ptr(), 1.0
    );

    assert_close(forces_sum(scalar), forces_sum(vector));
    assert!(forces_sum(scalar)[1].abs() > 0.0);

    simulation_result_drop(scalar);
    simulation_result_drop(vector);

    // Optimal steady state
    let scalar = complete_sail_model_simulate_optimal_steady_state_condition(model, wind, SHIP_VELOCITY, 1.0);
    let vector = complete_sail_model_simulate_optimal_steady_state_condition_with_ship_velocity_vector(
        model, wind, ship_velocity_vector.as_ptr(), 1.0
    );

    assert_close(forces_sum(scalar), forces_sum(vector));

    simulation_result_drop(scalar);
    simulation_result_drop(vector);

    // Freestream velocity
    let nr_points = complete_sail_model_get_number_of_freestream_velocity_points(model) as usize;

    let mut freestream_scalar = vec![0.0; 3 * nr_points];
    let mut freestream_vector = vec![0.0; 3 * nr_points];

    assert_eq!(
        complete_sail_model_freestream_velocity(
            model, wind, SHIP_VELOCITY, 0.0, freestream_scalar.as_mut_ptr(), nr_points
        ),
        nr_points as i32
    );

    assert_eq!(
        complete_sail_model_freestream_velocity_with_ship_velocity_vector(
            model, wind, ship_velocity_vector.as_ptr(), 0.0, freestream_vector.as_mut_ptr(), nr_points
        ),
        nr_points as i32
    );

    assert_eq!(freestream_scalar, freestream_vector);

    // A single time step
    let scalar = complete_sail_model_do_step(model, 0.0, 1.0, wind, SHIP_VELOCITY);
    let vector = complete_sail_model_do_step_with_ship_velocity_vector(
        model, 0.0, 1.0, wind, ship_velocity_vector.as_ptr()
    );

    assert_close(forces_sum(scalar), forces_sum(vector));

    simulation_result_drop(scalar);
    simulation_result_drop(vector);

    // Null pointers
    stormbird_clear_last_error();

    assert!(complete_sail_model_do_step_with_ship_velocity_vector(
        model, 0.0, 1.0, wind, std::ptr::null()
    ).is_null());

    assert!(last_error_message().unwrap().contains("do_step_with_ship_velocity_vector"));

    wind_condition_drop(wind);
    complete_sail_model_drop(model);
}

#[test]
fn span_varying_internal_states_can_be_set() {
    let model = new_model(None);

    let locations = [-0.5, 0.5];
    let values = [1.0, 3.0];

    assert_eq!(
        complete_sail_model_set_internal_state_linear(model, 1, locations.as_ptr(), values.as_ptr(), 2),
        0
    );

    let mut internal_state = [0.0; 2];

    assert_eq!(complete_sail_model_internal_state(model, internal_state.as_mut_ptr(), 2), 2);

    // The representative value is the mean over the span
    assert!((internal_state[1] - 2.0).abs() < TOLERANCE, "{:?}", internal_state);

    let boundaries = [-0.5, 0.0, 0.5];
    let zone_values = [1.0, 5.0];

    assert_eq!(
        complete_sail_model_set_internal_state_piecewise_constant(
            model, 0, boundaries.as_ptr(), 3, zone_values.as_ptr(), 2
        ),
        0
    );

    assert_eq!(complete_sail_model_internal_state(model, internal_state.as_mut_ptr(), 2), 2);
    assert!((internal_state[0] - 3.0).abs() < TOLERANCE, "{:?}", internal_state);

    // Errors
    stormbird_clear_last_error();

    assert_eq!(
        complete_sail_model_set_internal_state_linear(model, 2, locations.as_ptr(), values.as_ptr(), 2),
        -6
    );

    assert!(last_error_message().unwrap().contains("out of range"));

    let decreasing = [0.5, -0.5];

    assert_eq!(
        complete_sail_model_set_internal_state_linear(model, 0, decreasing.as_ptr(), values.as_ptr(), 2),
        -6
    );

    assert!(last_error_message().unwrap().contains("increasing"));

    assert_eq!(
        complete_sail_model_set_internal_state_piecewise_constant(
            model, 0, boundaries.as_ptr(), 3, zone_values.as_ptr(), 1
        ),
        -6
    );

    assert_eq!(
        complete_sail_model_set_internal_state_linear(model, 0, std::ptr::null(), values.as_ptr(), 2),
        -1
    );

    assert_eq!(
        complete_sail_model_set_internal_state_linear(model, 0, locations.as_ptr(), values.as_ptr(), 0),
        -2
    );

    complete_sail_model_drop(model);
}

#[test]
fn the_simulation_result_can_be_queried() {
    let model = new_model(None);
    let wind = beam_wind();

    let angles = [0.2, 0.2];
    assert_eq!(complete_sail_model_set_local_wing_angles(model, angles.as_ptr(), 2), 0);

    let result = complete_sail_model_do_step(model, 0.0, 1.0, wind, SHIP_VELOCITY);

    assert!(!result.is_null());

    let nr_wings = simulation_result_nr_of_wings(result) as usize;
    let nr_span_lines = simulation_result_nr_of_span_lines(result) as usize;

    assert_eq!(nr_wings, 2);
    assert_eq!(nr_span_lines, complete_sail_model_get_number_of_span_lines(model) as usize);

    assert!(simulation_result_iterations(result) >= 1);
    assert!(simulation_result_residual(result).is_finite());

    // Wing indices cover all the line elements
    let mut wing_indices = vec![0usize; 2 * nr_wings];

    assert_eq!(simulation_result_wing_indices(result, wing_indices.as_mut_ptr(), nr_wings), 2);
    assert_eq!(wing_indices[0], 0);
    assert_eq!(wing_indices[1], wing_indices[2]);
    assert_eq!(wing_indices[3], nr_span_lines);

    // The integrated values of each wing sum to the total
    let mut integrated_forces = vec![IntegratedValues::default(); nr_wings];

    assert_eq!(
        simulation_result_integrated_forces(result, integrated_forces.as_mut_ptr(), nr_wings),
        2
    );

    let mut sum = [0.0; 3];

    for values in &integrated_forces {
        for i in 0..3 {
            sum[i] += values.total[i];
        }
    }

    assert_close(sum, forces_sum(result));

    let mut integrated_moments = vec![IntegratedValues::default(); nr_wings];

    assert_eq!(
        simulation_result_integrated_moments(result, integrated_moments.as_mut_ptr(), nr_wings),
        2
    );

    let mut input_power = vec![f64::NAN; nr_wings];

    assert_eq!(simulation_result_input_power(result, input_power.as_mut_ptr(), nr_wings), 2);
    assert!(input_power.iter().all(|power| *power == 0.0));

    // Arrays on each line element
    let mut ctrl_points = vec![0.0; 3 * nr_span_lines];
    let mut sectional_forces = vec![SectionalForces::default(); nr_span_lines];
    let mut circulation_strength = vec![0.0; nr_span_lines];
    let mut velocity = vec![0.0; 3 * nr_span_lines];
    let mut angles_of_attack = vec![0.0; nr_span_lines];
    let mut internal_state = vec![0.0; nr_span_lines];
    let mut amount_of_stall = vec![0.0; nr_span_lines];
    let mut acceleration = vec![0.0; 3 * nr_span_lines];
    let mut rotation_velocity = [f64::NAN; 3];

    let n = nr_span_lines as i32;

    assert_eq!(simulation_result_ctrl_points(result, ctrl_points.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_sectional_forces(result, sectional_forces.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_force_input_circulation_strength(result, circulation_strength.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_force_input_velocity(result, velocity.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_force_input_angles_of_attack(result, angles_of_attack.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_force_input_section_models_internal_state(result, internal_state.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_force_input_amount_of_stall(result, amount_of_stall.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_force_input_acceleration(result, acceleration.as_mut_ptr(), nr_span_lines), n);
    assert_eq!(simulation_result_force_input_rotation_velocity(result, rotation_velocity.as_mut_ptr()), 0);

    assert!(ctrl_points.iter().any(|v| *v != 0.0));
    assert!(sectional_forces.iter().any(|f| f.total[1] != 0.0));
    assert!(circulation_strength.iter().all(|v| *v != 0.0));
    assert!(velocity.iter().any(|v| *v != 0.0));
    assert!(angles_of_attack.iter().all(|v| v.is_finite() && *v != 0.0));
    assert!(amount_of_stall.iter().all(|v| (0.0..=1.0).contains(v)));
    assert_eq!(rotation_velocity, [0.0; 3]);

    // Errors
    assert_eq!(simulation_result_ctrl_points(std::ptr::null(), ctrl_points.as_mut_ptr(), 1), -1);
    assert_eq!(simulation_result_ctrl_points(result, ctrl_points.as_mut_ptr(), 0), -2);
    assert_eq!(simulation_result_iterations(std::ptr::null()), -1);
    assert!(simulation_result_residual(std::ptr::null()).is_nan());

    simulation_result_drop(result);
    wind_condition_drop(wind);
    complete_sail_model_drop(model);
}

#[test]
fn a_simulation_result_can_be_read_from_file() {
    let folder = std::env::temp_dir().join("cstormbird_tests");
    std::fs::create_dir_all(&folder).unwrap();

    let path = folder.join("result.json");

    let mut rust_result =
        stormbird::common_utils::results::simulation::SimulationResult::default();

    rust_result.time = 1.5;
    rust_result.iterations = 3;

    std::fs::write(&path, serde_json::to_string(&rust_result).unwrap()).unwrap();

    let path_c = CString::new(path.to_string_lossy().to_string()).unwrap();

    let result = simulation_result_new_from_json_file(path_c.as_ptr());

    assert!(!result.is_null(), "{:?}", last_error_message());
    assert_eq!(simulation_result_time(result), 1.5);
    assert_eq!(simulation_result_iterations(result), 3);

    simulation_result_drop(result);

    // A missing file and invalid content are reported
    stormbird_clear_last_error();

    let missing = CString::new("this_file_does_not_exist.json").unwrap();

    assert!(simulation_result_new_from_json_file(missing.as_ptr()).is_null());
    assert!(last_error_message().unwrap().contains("IO error"));

    std::fs::write(&path, "not json").unwrap();

    assert!(simulation_result_new_from_json_file(path_c.as_ptr()).is_null());
    assert!(last_error_message().unwrap().contains("JSON"));
}
