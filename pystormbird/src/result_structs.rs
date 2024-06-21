// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pyo3::types::PyType;

use stormbird::io_structs::result::SimulationResult as SimulationResultRust;
use stormbird::io_structs::forces_and_moments::SectionalForces as SectionalForcesRust;
use stormbird::io_structs::forces_and_moments::IntegratedValues as IntegratedValuesRust;
use stormbird::io_structs::forces_and_moments::SectionalForcesInput as SectionalForcesInputRust;

use crate::vec3::Vec3;

#[pyclass]
#[derive(Clone)]
pub struct SectionalForcesInput {
    pub data: SectionalForcesInputRust
}

#[pymethods]
impl SectionalForcesInput {
    #[getter]
    pub fn circulation_strength(&self) -> Vec<f64> {
        self.data.circulation_strength.clone()
    }

    #[getter]
    pub fn velocity(&self) -> Vec<Vec3> {
        self.data.velocity.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn angles_of_attack(&self) -> Vec<f64> {
        self.data.angles_of_attack.clone()
    }

    #[getter]
    pub fn acceleration(&self) -> Vec<Vec3> {
        self.data.acceleration.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn angles_of_attack_derivative(&self) -> Vec<f64> {
        self.data.angles_of_attack_derivative.clone()
    }

    #[getter]
    pub fn rotation_velocity(&self) -> Vec3 {
        Vec3::from(self.data.rotation_velocity.clone())
    }
}

#[pyclass]
#[derive(Clone)]
pub struct SectionalForces {
    pub data: SectionalForcesRust
}

#[pymethods]
impl SectionalForces {
    #[getter]
    pub fn circulatory(&self) -> Vec<Vec3> {
        self.data.circulatory.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn sectional_drag(&self) -> Vec<Vec3> {
        self.data.sectional_drag.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn added_mass(&self) -> Vec<Vec3> {
        self.data.added_mass.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn gyroscopic(&self) -> Vec<Vec3> {
        self.data.gyroscopic.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn total(&self) -> Vec<Vec3> {
        self.data.total.iter().map(|v| Vec3::from(v.clone())).collect()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct IntegratedValues {
    pub data: IntegratedValuesRust
}

#[pymethods]
impl IntegratedValues {
    #[getter]
    pub fn circulatory(&self) -> Vec3 {
        Vec3::from(self.data.circulatory.clone())
    }

    #[getter]
    pub fn sectional_drag(&self) -> Vec3 {
        Vec3::from(self.data.sectional_drag.clone())
    }

    #[getter]
    pub fn added_mass(&self) -> Vec3 {
        Vec3::from(self.data.added_mass.clone())
    }

    #[getter]
    pub fn gyroscopic(&self) -> Vec3 {
        Vec3::from(self.data.gyroscopic.clone())
    }

    #[getter]
    pub fn total(&self) -> Vec3 {
        Vec3::from(self.data.total.clone())
    }
}

#[pyclass]
#[derive(Clone)]
pub struct SimulationResult {
    pub data: SimulationResultRust
}

#[pymethods]
impl SimulationResult {
    #[classmethod]
    pub fn result_history_from_file(cls: &Bound<'_, PyType>, file_path: String) -> Vec<SimulationResult> {
        let rust_vector = SimulationResultRust::result_history_from_file(&file_path);

        rust_vector.iter().map(|v| SimulationResult { data: v.clone() }).collect()
    }

    #[getter]
    pub fn ctrl_points(&self) -> Vec<Vec3> {
        self.data.ctrl_points.iter().map(|v| Vec3::from(v.clone())).collect()
    }

    #[getter]
    pub fn force_input(&self) -> SectionalForcesInput {
        SectionalForcesInput { data: self.data.force_input.clone() }
    }

    #[getter]
    pub fn sectional_forces(&self) -> SectionalForces {
        SectionalForces { data: self.data.sectional_forces.clone() }
    }

    #[getter]
    pub fn integrated_forces(&self) -> Vec<IntegratedValues> {
        self.data.integrated_forces.iter().map(|v| IntegratedValues { data: v.clone() }).collect()
    }

    #[getter]
    pub fn integrated_moments(&self) -> Vec<IntegratedValues> {
        self.data.integrated_moments.iter().map(|v| IntegratedValues { data: v.clone() }).collect()
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

