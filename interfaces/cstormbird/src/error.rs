// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Storage of the message describing the last error, so that a C caller can get a description of
//! what went wrong in addition to the error code.

use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::c_char;

thread_local! {
    /// The message from the last function that failed on this thread.
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Stores the message describing an error that is about to be reported to the caller.
pub(crate) fn set_last_error(message: impl Into<Vec<u8>>) {
    let message = CString::new(message).unwrap_or_else(
        |_| c"Error message contained an interior null byte".to_owned()
    );

    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = Some(message);
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

            set_last_error(format!("Panic: {}", message));

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
            Some(message) => message.as_ptr(),
            None => std::ptr::null()
        }
    })
}

/// Clears the last error message on the calling thread, so that
/// `stormbird_last_error_message` returns NULL again.
#[unsafe(no_mangle)]
pub extern "C" fn stormbird_clear_last_error() {
    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = None;
    });
}
