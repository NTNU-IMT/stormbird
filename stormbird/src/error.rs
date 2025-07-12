
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! An implementation of a common error type that is used throughout the library.

use serde_json;
use std::fmt;

#[derive(Debug)]
/// A common error type intended to represent the various error that can occur while suing this
/// library.
pub enum Error {
    /// Interface to the standard library IO error
    IoError(std::io::Error),
    /// Interface to the Serde JSON error
    SerdeJsonError(serde_json::Error),
    /// A custom error that can be created from a string
    CustomStringError(String),
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
            Error::CustomStringError(error) => write!(f, "Custom string error: {}", error),
        }
    }
}

impl std::error::Error for Error {}