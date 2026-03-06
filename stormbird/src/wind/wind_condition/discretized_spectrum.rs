// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Deserialize, Serialize};

use stormath::type_aliases::Float;
use stormath::consts::TAU;

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Structure used to represent a spectrum as discrete harmonic signals
pub struct DiscretizedSpectrum {
    pub frequencies: Vec<Float>,
    pub amplitudes: Vec<Float>,
    pub phases: Vec<Float>,
}

impl DiscretizedSpectrum {
    pub fn from_json_string(json_string: &str) -> Self {
        serde_json::from_str(json_string).unwrap()
    }
    
    pub fn from_json_file(file_path: &str) -> Self {
        let json_string = std::fs::read_to_string(file_path).unwrap();
        serde_json::from_str(&json_string).unwrap()
    }
    
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    
    pub fn value_at_time(&self, time: Float) -> Float {
        let mut out = 0.0;
        
        for i in 0..self.frequencies.len() {
            out += self.amplitudes[i] * (TAU * self.frequencies[i] * time + self.phases[i]).sin();
        }
        
        out
    }
}
