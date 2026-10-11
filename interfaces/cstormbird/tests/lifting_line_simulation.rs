//! Tests of the C-interface to a lifting line simulation, exercising the functions the same way
//! a C caller would.

use std::ffi::{CStr, CString};

use cstormbird::error::{stormbird_clear_last_error, stormbird_last_error_message};
use cstormbird::lifting_line::simulation::*;
use cstormbird::results::*;

/// A quasi-steady setup with a single wing, used by most of the tests.
const QUASI_STEADY_SETUP: &str = r#"{
    "line_force_model": {
        "wing_builders": [
            {
                "section_points": [
                    {"x": 0.0, "y": 0.0, "z": 0.0},
                    {"x": 0.0, "y": 0.0, "z": 20.0}
                ],
                "chord_vectors": [
                    {"x": -2.0, "y": 0.0, "z": 0.0},
                    {"x": -2.0, "y": 0.0, "z": 0.0}
                ],
                "section_model": {"Foil": {}},
                "non_zero_circulation_at_ends": [false, false]
            }
        ],
        "nr_sections": 8,
        "density": 1.225,
        "local_wing_angles": [0.0]
    },
    "simulation_settings": {"QuasiSteady": {}}
}"#;

/// The last error message, read the same way a C caller would.
fn last_error_message() -> Option<String> {
    let message = stormbird_last_error_message();

    if message.is_null() {
        return None;
    }

    Some(unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned())
}

/// Builds the simulation used by the tests, and fails the test if the setup is rejected.
fn new_simulation(setup: &str) -> *mut LiftingLineSimulation {
    let setup_string = CString::new(setup).unwrap();

    let simulation = lifting_line_simulation_new(setup_string.as_ptr());

    assert!(
        !simulation.is_null(),
        "the setup was rejected: {}",
        last_error_message().unwrap_or_default()
    );

    simulation
}

/// A uniform freestream velocity at every point the simulation needs it.
fn uniform_freestream(simulation: *mut LiftingLineSimulation, velocity: [f64; 3]) -> Vec<f64> {
    let nr_points = lifting_line_simulation_get_number_of_freestream_velocity_points(simulation);

    assert!(nr_points > 0);

    velocity.iter().cycle().take(3 * nr_points as usize).copied().collect()
}

#[test]
fn a_simulation_can_be_built_and_queried() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    assert_eq!(lifting_line_simulation_get_number_of_sails(simulation), 1);
    assert_eq!(lifting_line_simulation_get_number_of_span_lines(simulation), 8);
    assert_eq!(lifting_line_simulation_has_dynamic_wake(simulation), 0);

    // A quasi-steady simulation only needs the freestream velocity at the control points
    assert_eq!(
        lifting_line_simulation_get_number_of_freestream_velocity_points(simulation),
        8
    );

    lifting_line_simulation_drop(simulation);
}

#[test]
fn the_freestream_velocity_points_follow_the_position_of_the_sails() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    let nr_points = 8;
    let mut points = vec![0.0; 3 * nr_points];

    assert_eq!(
        lifting_line_simulation_get_freestream_velocity_points(
            simulation,
            points.as_mut_ptr(),
            nr_points
        ),
        nr_points as i32
    );

    // The wing starts at the origin, so every control point is on the z-axis
    assert!(points[0].abs() < 1e-12);
    assert!(points[1].abs() < 1e-12);
    assert!(points[2] > 0.0);

    let translation = [100.0, 0.0, 0.0];

    assert_eq!(
        lifting_line_simulation_set_translation_only(simulation, translation.as_ptr()),
        0
    );

    let mut moved_points = vec![0.0; 3 * nr_points];

    lifting_line_simulation_get_freestream_velocity_points(
        simulation,
        moved_points.as_mut_ptr(),
        nr_points
    );

    for i in 0..nr_points {
        assert!((moved_points[3 * i] - points[3 * i] - 100.0).abs() < 1e-9);
    }

    lifting_line_simulation_drop(simulation);
}

#[test]
fn a_time_step_produces_forces_on_the_sail() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    // The wing is rotated relative to the flow, so it should produce a side force
    let angles = [10.0_f64.to_radians()];

    assert_eq!(
        lifting_line_simulation_set_local_wing_angles(simulation, angles.as_ptr(), angles.len()),
        0
    );

    let freestream = uniform_freestream(simulation, [10.0, 0.0, 0.0]);

    let result = lifting_line_simulation_do_step(
        simulation,
        0.0,
        1.0,
        freestream.as_ptr(),
        freestream.len() / 3
    );

    assert!(!result.is_null(), "{}", last_error_message().unwrap_or_default());

    assert_eq!(simulation_result_nr_of_wings(result), 1);
    assert!((simulation_result_time(result) - 0.0).abs() < 1e-12);

    let mut single_sail_results = [SingleSailResult::default(); 1];

    assert_eq!(
        simulation_result_as_simplified(result, single_sail_results.as_mut_ptr(), 1),
        1
    );

    let force = single_sail_results[0].force;

    // A lifting wing in a flow along the x-axis gives a force in the y-direction
    assert!(force[1].abs() > 1.0, "the side force was {}", force[1]);

    let mut total_force = [0.0; 3];

    assert_eq!(simulation_result_integrated_forces_sum(result, total_force.as_mut_ptr()), 0);

    // With a single sail the total equals the force on that sail
    for i in 0..3 {
        assert!((total_force[i] - force[i]).abs() < 1e-9);
    }

    simulation_result_drop(result);
    lifting_line_simulation_drop(simulation);
}

#[test]
fn the_wing_angle_changes_the_forces() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    let freestream = uniform_freestream(simulation, [10.0, 0.0, 0.0]);
    let nr_points = freestream.len() / 3;

    let side_force_at_angle = |angle: f64| {
        let angles = [angle];

        lifting_line_simulation_set_local_wing_angles(simulation, angles.as_ptr(), 1);

        let result = lifting_line_simulation_do_step(
            simulation,
            0.0,
            1.0,
            freestream.as_ptr(),
            nr_points
        );

        assert!(!result.is_null());

        let mut force = [0.0; 3];
        simulation_result_integrated_forces_sum(result, force.as_mut_ptr());

        simulation_result_drop(result);

        force[1]
    };

    let small_angle_force = side_force_at_angle(5.0_f64.to_radians());
    let large_angle_force = side_force_at_angle(10.0_f64.to_radians());

    assert!(
        large_angle_force.abs() > small_angle_force.abs(),
        "a larger angle should give a larger side force, but got {} and {}",
        small_angle_force,
        large_angle_force
    );

    // The angle that was set can be read back
    let mut angles_out = [0.0; 1];

    assert_eq!(
        lifting_line_simulation_local_wing_angles(simulation, angles_out.as_mut_ptr(), 1),
        1
    );

    assert!((angles_out[0] - 10.0_f64.to_radians()).abs() < 1e-12);

    lifting_line_simulation_drop(simulation);
}

#[test]
fn the_velocity_of_the_sails_is_felt_as_a_change_in_the_inflow() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    let angles = [10.0_f64.to_radians()];
    lifting_line_simulation_set_local_wing_angles(simulation, angles.as_ptr(), 1);

    let freestream = uniform_freestream(simulation, [10.0, 0.0, 0.0]);
    let nr_points = freestream.len() / 3;

    let result = lifting_line_simulation_do_step(
        simulation,
        0.0,
        1.0,
        freestream.as_ptr(),
        nr_points
    );

    let mut force_without_motion = [0.0; 3];
    simulation_result_integrated_forces_sum(result, force_without_motion.as_mut_ptr());
    simulation_result_drop(result);

    // Moving the sail along the flow direction reduces the felt velocity
    let velocity = [5.0, 0.0, 0.0];

    assert_eq!(
        lifting_line_simulation_set_velocity_linear(simulation, velocity.as_ptr()),
        0
    );

    let result = lifting_line_simulation_do_step(
        simulation,
        1.0,
        1.0,
        freestream.as_ptr(),
        nr_points
    );

    let mut force_with_motion = [0.0; 3];
    simulation_result_integrated_forces_sum(result, force_with_motion.as_mut_ptr());
    simulation_result_drop(result);

    assert!(
        force_with_motion[1].abs() < force_without_motion[1].abs(),
        "moving with the flow should reduce the force, but got {} and {}",
        force_without_motion[1],
        force_with_motion[1]
    );

    lifting_line_simulation_drop(simulation);
}

#[test]
fn a_wrong_number_of_freestream_velocity_points_is_reported() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    stormbird_clear_last_error();

    let too_few = vec![0.0; 3 * 4];

    let result = lifting_line_simulation_do_step(simulation, 0.0, 1.0, too_few.as_ptr(), 4);

    assert!(result.is_null());

    let message = last_error_message().expect("a failed call should store a message");

    assert!(message.contains("4 points"), "message was: {}", message);
    assert!(message.contains("8 points"), "message was: {}", message);

    lifting_line_simulation_drop(simulation);
}

#[test]
fn a_wrong_number_of_wing_angles_is_reported() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    stormbird_clear_last_error();

    let too_many = [0.1, 0.2];

    assert_eq!(
        lifting_line_simulation_set_local_wing_angles(
            simulation,
            too_many.as_ptr(),
            too_many.len()
        ),
        -2
    );

    let message = last_error_message().expect("a failed call should store a message");

    assert!(message.contains("2 angles"), "message was: {}", message);
    assert!(message.contains("1 sails"), "message was: {}", message);

    lifting_line_simulation_drop(simulation);
}

#[test]
fn induced_velocities_are_rejected_for_a_quasi_steady_wake() {
    let simulation = new_simulation(QUASI_STEADY_SETUP);

    stormbird_clear_last_error();

    let points = [0.0, 0.0, 10.0];
    let mut velocity_out = [0.0; 3];

    assert_eq!(
        lifting_line_simulation_induced_velocities(
            simulation,
            points.as_ptr(),
            1,
            velocity_out.as_mut_ptr()
        ),
        -5
    );

    let message = last_error_message().expect("a failed call should store a message");

    assert!(message.contains("dynamic"), "message was: {}", message);

    lifting_line_simulation_drop(simulation);
}

#[test]
fn a_dynamic_simulation_needs_the_freestream_velocity_in_the_wake_as_well() {
    let setup = QUASI_STEADY_SETUP.replace(
        r#""simulation_settings": {"QuasiSteady": {}}"#,
        r#""simulation_settings": {"Dynamic": {"wake": {"nr_panels_per_line_element": 4}}}"#
    );

    let simulation = new_simulation(&setup);

    assert_eq!(lifting_line_simulation_has_dynamic_wake(simulation), 1);

    // The wake points come in addition to the control points
    let nr_points = lifting_line_simulation_get_number_of_freestream_velocity_points(simulation);

    assert!(nr_points > 8, "the dynamic wake should add points, but got {}", nr_points);

    let freestream = uniform_freestream(simulation, [10.0, 0.0, 0.0]);

    let result = lifting_line_simulation_do_step(
        simulation,
        0.0,
        1.0,
        freestream.as_ptr(),
        nr_points as usize
    );

    assert!(!result.is_null(), "{}", last_error_message().unwrap_or_default());

    simulation_result_drop(result);

    // The induced velocities are available with a dynamic wake
    let points = [50.0, 0.0, 10.0];
    let mut velocity_out = [0.0; 3];

    assert_eq!(
        lifting_line_simulation_induced_velocities(
            simulation,
            points.as_ptr(),
            1,
            velocity_out.as_mut_ptr()
        ),
        1
    );

    lifting_line_simulation_drop(simulation);
}
