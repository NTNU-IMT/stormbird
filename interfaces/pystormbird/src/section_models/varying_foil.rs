// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pythonize::pythonize;

use stormbird::section_models::varying_foil::VaryingFoil as VaryingFoilRust;
use stormbird::section_models::SectionModel as SectionModelRust;

use super::SectionModel;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct VaryingFoil {
    pub data: VaryingFoilRust
}

#[pymethods]
impl VaryingFoil {
    #[new]
    pub fn new(input_str: String) -> PyResult<Self> {
        let data = VaryingFoilRust::new_from_string(&input_str).map_err(|e| {
            PyValueError::new_err(format!("Failed to parse varying foil from string: {}", e))
        })?;

        Ok(Self { data })
    }

    pub fn lift_coefficient(&self, angle_of_attack: f64, internal_state: f64) -> f64 {
        self.data.lift_coefficient(angle_of_attack, internal_state)
    }

    pub fn drag_coefficient(&self, angle_of_attack: f64, internal_state: f64) -> f64 {
        self.data.drag_coefficient(angle_of_attack, internal_state)
    }

    pub fn as_multi_element_foil(&self) -> SectionModel {
        SectionModel {
            data: SectionModelRust::MultiElementFoil(self.data.clone())
        }
    }

    pub fn __str__(&self) -> String {
        self.data.to_string()
    }

    #[getter]
    /// Uses the built in json module to convert the string to a dictionary
    pub fn __dict__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(pythonize(py, &self.data)?)
    }

}
