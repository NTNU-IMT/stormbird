//! Tests of the C-interface to the wind condition, exercising the functions the same way a C
//! caller would.

use std::ffi::{CStr, CString};

use cstormbird::error::{stormbird_clear_last_error, stormbird_last_error_message};
use cstormbird::wind::*;

const TOLERANCE: f64 = 1e-12;

/// The last error message, read the same way a C caller would.
fn last_error_message() -> Option<String> {
    let message = stormbird_last_error_message();

    if message.is_null() {
        return None;
    }

    Some(unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned())
}

#[test]
fn constant_condition_has_the_same_velocity_at_all_heights() {
    let condition = wind_condition_new_constant(1.5, 10.0);

    assert!(!condition.is_null());

    assert!((wind_condition_get_direction_coming_from(condition) - 1.5).abs() < TOLERANCE);
    assert!((wind_condition_steady_true_wind_velocity_at_height(condition, 5.0) - 10.0).abs() < TOLERANCE);
    assert!((wind_condition_steady_true_wind_velocity_at_height(condition, 50.0) - 10.0).abs() < TOLERANCE);

    wind_condition_drop(condition);
}

#[test]
fn power_model_returns_the_reference_velocity_at_the_reference_height() {
    let condition = wind_condition_new_power_model_with_default_shape(0.0, 12.0);

    let velocity_at_reference_height = wind_condition_steady_true_wind_velocity_at_height(
        condition,
        10.0
    );

    assert!((velocity_at_reference_height - 12.0).abs() < TOLERANCE);

    // The default profile increases with height
    assert!(wind_condition_steady_true_wind_velocity_at_height(condition, 40.0) > 12.0);

    wind_condition_drop(condition);
}

#[test]
fn logarithmic_model_getters_and_setters_work_on_the_right_variant() {
    let condition = wind_condition_new_logarithmic_model(0.0, 0.5, 0.001, 0.0);

    assert!((wind_condition_get_friction_velocity(condition) - 0.5).abs() < TOLERANCE);
    assert!((wind_condition_get_surface_roughness(condition) - 0.001).abs() < TOLERANCE);
    // An Obukhov length of zero means no stability correction
    assert!(wind_condition_get_obukhov_length(condition).abs() < TOLERANCE);
    assert!(wind_condition_businger_dyer_unscaled_correction(condition, 10.0).abs() < TOLERANCE);
    assert!((wind_condition_get_von_karman_constant(condition) - 0.41).abs() < TOLERANCE);

    assert_eq!(wind_condition_set_friction_velocity(condition, 0.6), 0);
    assert!((wind_condition_get_friction_velocity(condition) - 0.6).abs() < TOLERANCE);

    assert_eq!(wind_condition_set_obukhov_length(condition, 100.0), 0);
    assert!((wind_condition_get_obukhov_length(condition) - 100.0).abs() < TOLERANCE);
    // A stable atmosphere now gives a non-zero correction
    assert!(wind_condition_businger_dyer_unscaled_correction(condition, 10.0).abs() > TOLERANCE);

    wind_condition_drop(condition);
}

#[test]
fn logarithmic_setters_are_rejected_for_other_variants() {
    let condition = wind_condition_new_constant(0.0, 10.0);

    assert_eq!(wind_condition_set_friction_velocity(condition, 0.6), -4);
    assert!(wind_condition_get_friction_velocity(condition).abs() < TOLERANCE);

    wind_condition_drop(condition);
}

#[test]
fn gust_spectrum_can_be_set_from_arrays() {
    let condition = wind_condition_new_constant(0.0, 10.0);

    let frequencies = [0.1, 0.2];
    let amplitudes = [1.0, 0.5];
    let phases = [0.0, 0.0];

    let status = wind_condition_set_parallel_gust(
        condition,
        frequencies.as_ptr(),
        amplitudes.as_ptr(),
        phases.as_ptr(),
        frequencies.len()
    );

    assert_eq!(status, 0);

    // At time zero all the harmonic components are zero, so the velocity is the steady one
    let velocity_at_zero = wind_condition_unsteady_parallel_true_wind_velocity_at_height(
        condition,
        10.0,
        0.0
    );

    assert!((velocity_at_zero - 10.0).abs() < TOLERANCE);

    // At a quarter of the period of the first component the gust contributes
    let velocity_later = wind_condition_unsteady_parallel_true_wind_velocity_at_height(
        condition,
        10.0,
        2.5
    );

    assert!((velocity_later - 10.0).abs() > 0.1);

    wind_condition_drop(condition);
}

#[test]
fn gust_spectrum_can_be_set_from_a_json_string() {
    let condition = wind_condition_new_constant(0.0, 10.0);

    let gust_string = CString::new(
        r#"{"frequencies": [0.1], "amplitudes": [2.0], "phases": [1.5707963267948966]}"#
    ).unwrap();

    let status = wind_condition_set_perpendicular_gust_from_json_string(
        condition,
        gust_string.as_ptr()
    );

    assert_eq!(status, 0);

    // With a phase shift of pi/2 the component is at its maximum at time zero
    let velocity = wind_condition_unsteady_perpendicular_true_wind_velocity(condition, 0.0);

    assert!((velocity - 2.0).abs() < 1e-9);

    wind_condition_drop(condition);
}

#[test]
fn invalid_gust_json_is_reported_instead_of_panicking() {
    let condition = wind_condition_new_constant(0.0, 10.0);

    let gust_string = CString::new(r#"{"not_a_spectrum": true}"#).unwrap();

    let status = wind_condition_set_vertical_gust_from_json_string(
        condition,
        gust_string.as_ptr()
    );

    assert_eq!(status, -3);

    // The condition is unchanged, so there is still no vertical gust
    assert!(wind_condition_unsteady_vertical_true_wind_velocity(condition, 1.0).abs() < TOLERANCE);

    wind_condition_drop(condition);
}

#[test]
fn condition_can_be_built_from_a_json_string() {
    let condition_string = CString::new(
        r#"{
            "direction_coming_from": 0.5,
            "velocity_variation": {"PowerModel": {"reference_velocity": 8.0}},
            "parallel_gust": {"frequencies": [0.1], "amplitudes": [1.0], "phases": [0.0]}
        }"#
    ).unwrap();

    let condition = wind_condition_new_from_json_string(condition_string.as_ptr());

    assert!(!condition.is_null());

    assert!((wind_condition_get_direction_coming_from(condition) - 0.5).abs() < TOLERANCE);
    assert!((wind_condition_steady_true_wind_velocity_at_height(condition, 10.0) - 8.0).abs() < TOLERANCE);

    wind_condition_drop(condition);
}

#[test]
fn invalid_json_returns_a_null_pointer() {
    let condition_string = CString::new(r#"{"direction_coming_from": 0.5}"#).unwrap();

    let condition = wind_condition_new_from_json_string(condition_string.as_ptr());

    assert!(condition.is_null());

    // Dropping a null pointer is allowed
    wind_condition_drop(condition);
}

#[test]
fn null_pointers_are_handled_by_all_the_accessors() {
    let null_condition = std::ptr::null_mut();

    assert!(wind_condition_get_direction_coming_from(null_condition).is_nan());
    assert!(wind_condition_steady_true_wind_velocity_at_height(null_condition, 10.0).is_nan());
    assert!(wind_condition_get_friction_velocity(null_condition).is_nan());

    assert_eq!(wind_condition_set_direction_coming_from(null_condition, 1.0), -1);
    assert_eq!(wind_condition_set_friction_velocity(null_condition, 1.0), -1);
    assert_eq!(
        wind_condition_set_parallel_gust(null_condition, std::ptr::null(), std::ptr::null(), std::ptr::null(), 1),
        -1
    );

    assert!(wind_condition_new_from_json_string(std::ptr::null()).is_null());
}

#[test]
fn failures_describe_themselves_through_the_last_error_message() {
    stormbird_clear_last_error();

    assert_eq!(last_error_message(), None);

    // An error that comes from the library itself
    let condition_string = CString::new(r#"{"direction_coming_from": 0.5}"#).unwrap();

    assert!(wind_condition_new_from_json_string(condition_string.as_ptr()).is_null());

    let message = last_error_message().expect("a failed call should store a message");

    assert!(message.contains("wind_condition_new_from_json_string"), "message was: {}", message);
    assert!(message.contains("velocity_variation"), "message was: {}", message);

    // A later failure replaces the message
    let condition = wind_condition_new_constant(0.0, 10.0);

    assert_eq!(wind_condition_set_friction_velocity(condition, 0.6), -4);

    let message = last_error_message().expect("a failed call should store a message");

    assert!(message.contains("wind_condition_set_friction_velocity"), "message was: {}", message);
    assert!(message.contains("logarithmic"), "message was: {}", message);

    stormbird_clear_last_error();

    assert_eq!(last_error_message(), None);

    // A successful call does not store a message
    assert_eq!(wind_condition_set_direction_coming_from(condition, 1.0), 0);
    assert_eq!(last_error_message(), None);

    wind_condition_drop(condition);
}

#[test]
fn an_empty_gust_spectrum_is_rejected() {
    let condition = wind_condition_new_constant(0.0, 10.0);

    let empty: [f64; 0] = [];

    let status = wind_condition_set_parallel_gust(
        condition,
        empty.as_ptr(),
        empty.as_ptr(),
        empty.as_ptr(),
        0
    );

    assert_eq!(status, -2);

    wind_condition_drop(condition);
}
