// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormbird::section_models::varying_foil::VaryingFoil as VaryingFoilRust;

#[pyclass]
#[derive(Clone)]
pub struct VaryingFoil {
    pub data: VaryingFoilRust
}

#[pymethods]
impl VaryingFoil {
    #[new]
    pub fn new(input_str: String) -> Self {
        Self {
            data: VaryingFoilRust::new_from_string(&input_str)
        }
    }

    pub fn set_internal_state(&mut self, internal_state: f64) {
        self.data.current_internal_state = internal_state;
    }

    pub fn lift_coefficient(&self, angle_of_attack: f64) -> f64 {
        self.data.lift_coefficient(angle_of_attack)
    }

    pub fn drag_coefficient(&self, angle_of_attack: f64) -> f64 {
        self.data.drag_coefficient(angle_of_attack)
    }

    pub fn __str__(&self) -> String {
        self.data.to_string()
    }

    #[getter]
    /// Uses the built in json module to convert the string to a dictionary
    pub fn __dict__(&self) -> PyResult<PyObject> {
        let json_string = self.__str__();
        
        Python::with_gil(|py| {
            let json_module = PyModule::import_bound(py, "json")?;

            let dict = json_module.call_method1("loads", (json_string,))?;

            Ok(dict.into())
        })
    }

}