// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A Python interface to the Stormbird library. 

use pyo3::prelude::*;

use pyo3::{
    wrap_pymodule,
    types::PyDict
};

mod section_models;
mod result_structs;
mod line_force_model;
mod lifting_line;

#[pymodule]
fn pystormbird(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<result_structs::SimulationResult>()?;
    m.add_class::<result_structs::SectionalForcesInput>()?;
    
    m.add_wrapped(wrap_pymodule!(section_models::section_models))?;
    m.add_wrapped(wrap_pymodule!(line_force_model::line_force_model))?;
    m.add_wrapped(wrap_pymodule!(lifting_line::lifting_line))?;
    
    // Trick to make the module visible to Python. Taken from:
    // <https://medium.com/@kudryavtsev_ia/how-i-design-and-develop-real-world-python-extensions-in-rust-2abfe2377182>
    Python::with_gil(|py| {
        let sys = PyModule::import_bound(py, "sys")?;
        let sys_modules: &PyDict = sys
            .getattr("modules")?
            .extract()?;

        sys_modules.set_item("pystormbird.section_models",   m.getattr("section_models")?)?;
        sys_modules.set_item("pystormbird.line_force_model", m.getattr("line_force_model")?)?;
        sys_modules.set_item("pystormbird.lifting_line",     m.getattr("lifting_line")?)?;


        Ok(())
    })
}