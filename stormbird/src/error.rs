
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! An implementation of a common error type that is used throughout the library.

use serde_json;
use std::fmt;

#[derive(Debug)]
/// A common error type intended to represent the various errors that can occur while using this
/// library.
pub enum Error {
    /// Interface to the standard library IO error
    IoError(std::io::Error),
    /// Interface to the Serde JSON error
    SerdeJsonError(serde_json::Error),
    /// Invalid input or settings, for instance inconsistent data in a builder. The string describes
    /// what is wrong with the input.
    InvalidInput(String),
    /// A custom error that can be created from a string
    CustomStringError(String),
}

impl Error {
    /// Creates an [`Error::InvalidInput`] from anything that can be converted to a string
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Error::InvalidInput(message.into())
    }

    /// Returns a numeric code for the type of error. This is useful for interfaces that can only
    /// report errors as numbers, such as the FMU version of the library. The codes are:
    ///
    /// | Code | Error                           |
    /// |------|---------------------------------|
    /// | 1    | [`Error::IoError`]              |
    /// | 2    | [`Error::SerdeJsonError`]       |
    /// | 3    | [`Error::InvalidInput`]         |
    /// | 4    | [`Error::CustomStringError`]    |
    ///
    /// Zero is never used, so that it can represent *no error* in the interfaces.
    pub fn code(&self) -> i32 {
        match self {
            Error::IoError(_) => 1,
            Error::SerdeJsonError(_) => 2,
            Error::InvalidInput(_) => 3,
            Error::CustomStringError(_) => 4,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::SerdeJsonError(error)
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error::CustomStringError(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(error) => write!(f, "IO error: {}", error),
            Error::SerdeJsonError(error) => write!(f, "Serde JSON error: {}", error),
            Error::InvalidInput(error) => write!(f, "Invalid input: {}", error),
            Error::CustomStringError(error) => write!(f, "Custom string error: {}", error),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError(error) => Some(error),
            Error::SerdeJsonError(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_unique_and_non_zero() {
        let errors = [
            Error::IoError(std::io::Error::other("io")),
            Error::SerdeJsonError(serde_json::from_str::<i32>("not json").unwrap_err()),
            Error::invalid_input("invalid"),
            Error::CustomStringError("custom".to_string()),
        ];

        let codes: Vec<i32> = errors.iter().map(|error| error.code()).collect();

        assert_eq!(codes, vec![1, 2, 3, 4]);
    }
}
