// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

pub mod foil;
pub mod rotating_cylinder;
pub mod varying_foil;

use stormbird::section_models::SectionModel as SectionModelRust;

#[pyclass]
#[derive(Clone)]
pub struct SectionModel{
    pub data: SectionModelRust,   
}

#[pymethods]
impl SectionModel {
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

#[pymodule]
pub fn section_models(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SectionModel>()?;
    m.add_class::<foil::Foil>()?;
    m.add_class::<varying_foil::VaryingFoil>()?;
    m.add_class::<rotating_cylinder::RotatingCylinder>()?;
    Ok(())
}