// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! The error codes of the C interface, and storage of the code and message of the last error, so
//! that a C caller can get a description of what went wrong in addition to the error code.
//!
//! The error codes are the same as in the rest of Stormbird. The codes from 1 to 4 are the codes
//! of the errors in Stormbird, see `stormbird::error::Error::code`, the codes from 10 to 19 are
//! used by the FMU, and the codes from 20 to 29 are specific to the C interface.

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::c_char;

/// No error
pub const STORMBIRD_OK: i32 = 0;
/// IO error, e.g., a file that can not be read. Same as in Stormbird.
pub const STORMBIRD_ERROR_IO: i32 = 1;
/// JSON error, e.g., a setup string with a syntax error or an unknown field. Same as in Stormbird.
pub const STORMBIRD_ERROR_JSON: i32 = 2;
/// Invalid input, e.g., inconsistent settings, an array with the wrong length, a string that is not
/// valid UTF-8, or a wing index that is out of range. Same as in Stormbird.
pub const STORMBIRD_ERROR_INVALID_INPUT: i32 = 3;
/// Other error from Stormbird. Same as in Stormbird.
pub const STORMBIRD_ERROR_OTHER: i32 = 4;
/// One of the input pointers is null. Specific to the C interface.
pub const STORMBIRD_ERROR_NULL_POINTER: i32 = 20;
/// An unexpected error (a panic) was caught. Indicates a bug, and should be reported. Specific to
/// the C interface.
pub const STORMBIRD_ERROR_UNEXPECTED: i32 = 21;

/// The code and message of an error
struct LastError {
    code: i32,
    message: CString,
}

thread_local! {
    /// The error from the last function that failed on this thread.
    static LAST_ERROR: RefCell<Option<LastError>> = const { RefCell::new(None) };
}

/// Stores the code and the message describing an error that is about to be reported to the caller.
pub(crate) fn set_last_error(code: i32, message: impl Into<Vec<u8>>) {
    let message = CString::new(message).unwrap_or_else(
        |_| c"Error message contained an interior null byte".to_owned()
    );

    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = Some(LastError { code, message });
    });
}

/// Runs a closure that may panic, and stores the panic message as the last error if it does.
///
/// The library is used through a C-ABI, where a panic that unwinds out of a function aborts the
/// whole process. Entry points that run code that may panic on invalid input use this to report
/// the problem as an error instead.
pub(crate) fn catch_panic<T>(operation: impl FnOnce() -> T + std::panic::UnwindSafe) -> Option<T> {
    // The default panic hook prints the panic to stderr, which is noise when the panic is handled
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let result = std::panic::catch_unwind(operation);

    std::panic::set_hook(previous_hook);

    match result {
        Ok(value) => Some(value),
        Err(payload) => {
            let message = if let Some(message) = payload.downcast_ref::<&str>() {
                (*message).to_string()
            } else if let Some(message) = payload.downcast_ref::<String>() {
                message.clone()
            } else {
                "Unknown panic".to_string()
            };

            set_last_error(STORMBIRD_ERROR_UNEXPECTED, format!("Panic: {}", message));

            None
        }
    }
}

/// A description of the last error that occurred on the calling thread, or NULL if no function
/// has failed yet.
///
/// The message describes the most recent *failed* call, so it should only be read after a
/// function has reported a failure through its return value. Successful calls do not change it.
///
/// # Safety
/// - The returned string is owned by the library, and is only valid until the next failing call
///   on the same thread, or until `stormbird_clear_last_error` is called. Copy it if it is needed
///   after that.
#[unsafe(no_mangle)]
pub extern "C" fn stormbird_last_error_message() -> *const c_char {
    LAST_ERROR.with(|last_error| {
        match last_error.borrow().as_ref() {
            Some(error) => error.message.as_ptr(),
            None => std::ptr::null()
        }
    })
}

/// The code of the last error that occurred on the calling thread, or `STORMBIRD_OK` (0) if no
/// function has failed yet. See the `STORMBIRD_ERROR_*` constants for the meaning of each code.
///
/// As for `stormbird_last_error_message`, the code describes the most recent *failed* call, so it
/// should only be read after a function has reported a failure. Successful calls do not change it.
#[unsafe(no_mangle)]
pub extern "C" fn stormbird_last_error_code() -> i32 {
    LAST_ERROR.with(|last_error| {
        match last_error.borrow().as_ref() {
            Some(error) => error.code,
            None => STORMBIRD_OK,
        }
    })
}

/// Clears the last error on the calling thread, so that `stormbird_last_error_message` returns
/// NULL and `stormbird_last_error_code` returns `STORMBIRD_OK` again.
#[unsafe(no_mangle)]
pub extern "C" fn stormbird_clear_last_error() {
    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = None;
    });
}
