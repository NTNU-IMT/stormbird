use serde_json;
use std::fmt;

#[derive(Debug)]
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