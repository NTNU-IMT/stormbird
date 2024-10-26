// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

pub mod symmetry_condition;

use stormbird::lifting_line::singularity_elements::PotentialTheorySettings as PotentialTheorySettingsRust;

#[pyclass]
pub struct PotentialTheorySettings {
    pub data: PotentialTheorySettingsRust
}

#[pymethods]
impl PotentialTheorySettings {
    #[new]
    #[pyo3(
        signature = (
            far_field_ratio = PotentialTheorySettingsRust::default_far_field_ratio()
        )
    )]
    fn new(far_field_ratio: f64) -> Self {
        Self {
            data: PotentialTheorySettingsRust {
                far_field_ratio,
                ..Default::default()
            }
        }
    }

    #[getter]
    fn far_field_ratio(&self) -> f64 {
        self.data.far_field_ratio
    }
}