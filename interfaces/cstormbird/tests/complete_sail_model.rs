//! Tests of the error handling in the C-interface to the complete sail model.

use std::ffi::{CStr, CString};

use cstormbird::error::{stormbird_clear_last_error, stormbird_last_error_message};
use cstormbird::lifting_line::complete_sail_model::*;

/// The last error message, read the same way a C caller would.
fn last_error_message() -> Option<String> {
    let message = stormbird_last_error_message();

    if message.is_null() {
        return None;
    }

    Some(unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned())
}

#[test]
fn a_null_setup_string_is_reported() {
    stormbird_clear_last_error();

    assert!(complete_sail_model_new(std::ptr::null()).is_null());

    let message = last_error_message().expect("a failed call should store a message");

    assert!(message.contains("complete_sail_model_new"), "message was: {}", message);
    assert!(message.contains("null"), "message was: {}", message);
}

#[test]
fn an_invalid_setup_string_is_reported() {
    stormbird_clear_last_error();

    let setup_string = CString::new("{ this is not valid json").unwrap();

    assert!(complete_sail_model_new(setup_string.as_ptr()).is_null());

    let message = last_error_message().expect("a failed call should store a message");

    assert!(message.contains("complete_sail_model_new"), "message was: {}", message);
    // The message from serde tells the caller where the problem is
    assert!(message.contains("line 1"), "message was: {}", message);
}

#[test]
fn a_setup_string_missing_fields_is_reported() {
    stormbird_clear_last_error();

    let setup_string = CString::new(r#"{"wind_environment": {}}"#).unwrap();

    assert!(complete_sail_model_new(setup_string.as_ptr()).is_null());

    let message = last_error_message().expect("a failed call should store a message");

    assert!(
        message.contains("lifting_line_simulation"),
        "the message should name the missing field, but was: {}",
        message
    );
}

#[test]
fn queries_on_a_null_model_are_reported() {
    stormbird_clear_last_error();

    assert_eq!(complete_sail_model_get_number_of_sails(std::ptr::null_mut()), -1);

    let message = last_error_message().expect("a failed call should store a message");

    assert!(
        message.contains("complete_sail_model_get_number_of_sails"),
        "message was: {}",
        message
    );

    // Dropping a null model is allowed, and does not report an error
    stormbird_clear_last_error();
    complete_sail_model_drop(std::ptr::null_mut());
    assert_eq!(last_error_message(), None);
}
