// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Conversion of errors from Stormbird to Python exceptions.

use pyo3::prelude::*;
use pyo3::exceptions::{PyIOError, PyValueError};

use stormbird::error::Error;

/// Converts an error from Stormbird to a Python exception, with the given context in front of the
/// error message. Errors from reading files become an `OSError`, while all other errors, i.e.,
/// invalid input, invalid JSON and other errors from Stormbird, become a `ValueError`.
pub fn to_py_err(context: &str, error: Error) -> PyErr {
    let message = format!("{}: {}", context, error);

    match error {
        Error::IoError(_) => PyIOError::new_err(message),
        _ => PyValueError::new_err(message),
    }
}
