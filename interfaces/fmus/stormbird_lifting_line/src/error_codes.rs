// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Error codes for the `error_code` output of the FMU.
//!
//! The FMU never crashes, or returns an error status to the importer, due to errors in the input or
//! in the simulation. Instead, the error is reported as a non-zero `error_code` output, and the
//! error message is printed to the standard error output. The codes from 1 to 9 are the same as
//! [`stormbird::error::Error::code`], while the codes from 10 and up are specific to the FMU.

use std::any::Any;
use std::fmt;

/// No error
pub const OK: i32 = 0;
/// A panic was caught while setting up the model
pub const PANIC_DURING_INITIALIZATION: i32 = 10;
/// A panic was caught while simulating a time step
pub const PANIC_DURING_TIME_STEP: i32 = 11;
/// The simulation gave forces or moments that are not finite, e.g., NaN
pub const NON_FINITE_RESULT: i32 = 12;
/// The sail model has more sails than the FMU has inputs and outputs for
pub const TOO_MANY_SAILS: i32 = 13;
/// A time step was requested, but the model was not set up
pub const MODEL_NOT_INITIALIZED: i32 = 14;
/// The input gave a freestream velocity that is not finite, e.g., NaN
pub const NON_FINITE_INPUT: i32 = 15;

#[derive(Debug, Clone)]
/// An error in the FMU, with the code that is given as output and a message that is printed
pub struct FmuError {
    pub code: i32,
    pub message: String,
}

impl FmuError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    /// Creates an error from a panic payload, as returned by `std::panic::catch_unwind`
    pub fn from_panic(code: i32, payload: Box<dyn Any + Send>) -> Self {
        let message = if let Some(message) = payload.downcast_ref::<&str>() {
            message.to_string()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            "unknown panic".to_string()
        };

        Self::new(code, format!("Panic: {}", message))
    }
}

impl From<stormbird::error::Error> for FmuError {
    fn from(error: stormbird::error::Error) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

impl fmt::Display for FmuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error code {}: {}", self.code, self.message)
    }
}
