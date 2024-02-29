// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pyo3::types::PyType;

use stormbird::lifting_line::singularity_elements::viscous_core_length::ViscousCoreLength as ViscousCoreLengthRust;

#[pyclass]
#[derive(Clone)]
pub struct ViscousCoreLength {
    pub data: ViscousCoreLengthRust
}

#[pymethods]
impl ViscousCoreLength {
    #[classmethod]
    pub fn relative(_cls: &PyType, relative_length: f64) -> Self {
        Self {
            data: ViscousCoreLengthRust::Relative(relative_length)
        }
    }

    #[classmethod]
    pub fn absolute(_cls: &PyType, absolute_length: f64) -> Self {
        Self {
            data: ViscousCoreLengthRust::Absolute(absolute_length)
        }
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.data)
    }
}

impl From<ViscousCoreLength> for ViscousCoreLengthRust {
    fn from(viscous_core_length: ViscousCoreLength) -> Self {
        viscous_core_length.data
    }
}

impl Default for ViscousCoreLength {
    fn default() -> Self {
        Self {
            data: ViscousCoreLengthRust::default()
        }
    }
}