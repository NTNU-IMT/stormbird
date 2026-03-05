// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormbird::wind::gust_spectrums::discretized_spectrum::DiscretizedSpectrum as DiscretizedSpectrumRust;

#[pyclass]
#[derive(Clone)]
pub struct DiscretizedSpectrum {
    pub data: DiscretizedSpectrumRust
}

#[pymethods]
impl DiscretizedSpectrum {
    #[new]
    pub fn new(setup_string: String) -> Self {
        Self{
            data: DiscretizedSpectrumRust::from_json_string(&setup_string)
        }
    }
    
    pub fn value_at_time(&self, time: f64) -> f64 {
        self.data.value_at_time(time)
    }
}
