// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

pub mod symmetry_condition;

use stormbird::lifting_line::singularity_elements::PotentialTheoryModel as PotentialTheoryModelRust;

#[pyclass]
pub struct PotentialTheoryModel {
    pub data: PotentialTheoryModelRust
}

#[pymethods]
impl PotentialTheoryModel {
    #[new]
    #[pyo3(
        signature = (
            closeness_error     = f64::MIN_POSITIVE, 
            far_field_ratio     = PotentialTheoryModelRust::default_far_field_ratio()
        )
    )]
    fn new(closeness_error: f64, far_field_ratio: f64) -> Self {
        Self {
            data: PotentialTheoryModelRust {
                closeness_error,
                far_field_ratio,
                ..Default::default()
            }
        }
    }

    #[getter]
    fn closeness_error(&self) -> f64 {
        self.data.closeness_error
    }

    #[getter]
    fn far_field_ratio(&self) -> f64 {
        self.data.far_field_ratio
    }
}