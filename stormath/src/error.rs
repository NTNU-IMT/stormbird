// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! General error types in the library.

use std::fmt;

#[derive(Debug)]
/// A common error type intended to represent the various error that can occur while suing this
/// library.
pub enum Error {
    NoSolution(String)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NoSolution(msg) => write!(f, "No solution found: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
