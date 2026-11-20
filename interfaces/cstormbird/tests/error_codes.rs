//! Tests of the error codes, which should be the same as in the rest of Stormbird.

use std::ffi::CString;

use cstormbird::error::*;
use cstormbird::lifting_line::complete_sail_model::*;
use cstormbird::results::*;

use stormbird::error::Error;

#[test]
fn the_codes_are_the_same_as_in_stormbird() {
    let io = Error::IoError(std::io::Error::other("io"));
    let json = Error::SerdeJsonError(serde_json::from_str::<i32>("not json").unwrap_err());
    let invalid = Error::InvalidInput("invalid".to_string());
    let other = Error::CustomStringError("other".to_string());

    assert_eq!(STORMBIRD_ERROR_IO, io.code());
    assert_eq!(STORMBIRD_ERROR_JSON, json.code());
    assert_eq!(STORMBIRD_ERROR_INVALID_INPUT, invalid.code());
    assert_eq!(STORMBIRD_ERROR_OTHER, other.code());

    // The codes specific to the C interface do not overlap with Stormbird or the FMU, which uses
    // the codes from 10 to 19
    assert!(STORMBIRD_ERROR_NULL_POINTER >= 20);
    assert!(STORMBIRD_ERROR_UNEXPECTED >= 20);
}

#[test]
fn the_last_error_code_describes_the_last_failure() {
    stormbird_clear_last_error();

    assert_eq!(stormbird_last_error_code(), STORMBIRD_OK);

    // Invalid JSON
    let invalid_json = CString::new("{ not json").unwrap();

    assert!(complete_sail_model_new(invalid_json.as_ptr()).is_null());
    assert_eq!(stormbird_last_error_code(), STORMBIRD_ERROR_JSON);

    // Valid JSON, but invalid settings, reported by Stormbird
    let invalid_settings = CString::new(r#"{
        "lifting_line_simulation": {
            "line_force_model": {
                "nr_sections": 5,
                "wing_builders": [{
                    "section_points": [{"x": 0.0, "y": 0.0, "z": 0.0}],
                    "chord_vectors": [{"x": 1.0, "y": 0.0, "z": 0.0}],
                    "section_model": {"SingleElementFoil": {}}
                }]
            }
        }
    }"#).unwrap();

    assert!(complete_sail_model_new(invalid_settings.as_ptr()).is_null());
    assert_eq!(stormbird_last_error_code(), STORMBIRD_ERROR_INVALID_INPUT);

    // A file that can not be read
    let missing_file = CString::new("this_file_does_not_exist.json").unwrap();

    assert!(simulation_result_new_from_json_file(missing_file.as_ptr()).is_null());
    assert_eq!(stormbird_last_error_code(), STORMBIRD_ERROR_IO);

    // A null pointer, where the returned code is the negative of the last error code
    let status = complete_sail_model_get_number_of_sails(std::ptr::null_mut());

    assert_eq!(status, -STORMBIRD_ERROR_NULL_POINTER);
    assert_eq!(stormbird_last_error_code(), STORMBIRD_ERROR_NULL_POINTER);

    // Clearing the error resets both the code and the message
    stormbird_clear_last_error();

    assert_eq!(stormbird_last_error_code(), STORMBIRD_OK);
    assert!(stormbird_last_error_message().is_null());
}
