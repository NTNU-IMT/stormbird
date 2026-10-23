// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! General error types in the library.

use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
/// A common error type intended to represent the various errors that can occur while using this
/// library.
pub enum Error {
    /// A solver or algorithm was not able to find a solution. The string describes why.
    NoSolution(String),
    /// The input is not valid. The string describes why.
    InvalidInput(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NoSolution(msg) => write!(f, "No solution found: {}", msg),
            Error::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
