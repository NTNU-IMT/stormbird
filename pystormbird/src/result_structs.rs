// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormbird::io_structs::result::SimulationResult as SimulationResultRust;

use crate::vec3::Vec3;

#[pyclass]
#[derive(Clone)]
pub struct SimulationResult {
    pub data: SimulationResultRust
}

#[pymethods]
impl SimulationResult {
    #[getter]
    pub fn ctrl_points(&self) -> Vec<Vec3> {
        self.data.ctrl_points.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn circulation_strength(&self) -> Vec<f64> {
        self.data.force_input.circulation_strength.clone()
    }

    #[getter]
    pub fn velocity(&self) -> Vec<Vec3> {
        self.data.force_input.velocity.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn sectional_forces(&self) -> Vec<Vec3> {
        self.data.sectional_forces.total.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn integrated_forces(&self) -> Vec<Vec3> {
        self.data.integrated_forces.iter().map(|v| Vec3::from(v.total.clone())).collect()
    }

    #[getter]
    pub fn integrated_moments(&self) -> Vec<Vec3> {
        self.data.integrated_moments.iter().map(|v| Vec3::from(v.total.clone())).collect()
    }

    #[getter]
    pub fn effective_angles_of_attack(&self) -> Vec<f64> {
        self.data.force_input.angles_of_attack.clone()
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.data)
    }

    pub fn integrated_forces_sum(&self) -> Vec3 {
        Vec3::from(self.data.integrated_forces_sum())
    }

    pub fn integrated_moments_sum(&self) -> Vec3 {
        Vec3::from(self.data.integrated_moments_sum())
    }
}

