// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;
use stormath::consts::TAU;
use crate::error::Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Structure used to represent a spectrum as discrete harmonic signals. The discrete variables must 
/// be set externally. Typically, they are generated from a gust spectrum.
pub struct DiscretizedSpectrum {
    /// The discrete frequencies in the spectrum
    pub frequencies: Vec<Float>,
    /// The amplitudes for each frequency
    pub amplitudes: Vec<Float>,
    /// The phase shift for each frequency
    pub phases: Vec<Float>,
}

impl DiscretizedSpectrum {
    /// Creates a spectrum from a JSON string
    pub fn from_json_string(json_string: &str) -> Result<Self, Error> {
        Ok(serde_json::from_str(json_string)?)
    }
    
    /// Creates a spectrum from a JSON file
    pub fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let json_string = std::fs::read_to_string(file_path)?;

        Self::from_json_string(&json_string)
    }
    
    /// Returns the spectrum as a JSON string
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("A discretized spectrum can always be serialized")
    }

    /// Computes the oscillating velocity as a function of time
    pub fn value_at_time(&self, time: Float) -> Float {
        let mut out = 0.0;
        
        for i in 0..self.frequencies.len() {
            out += self.amplitudes[i] * (TAU * self.frequencies[i] * time + self.phases[i]).sin();
        }
        
        out
    }
}
