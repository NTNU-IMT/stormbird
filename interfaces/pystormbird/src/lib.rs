// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A Python interface to the Stormbird library. 

use pyo3::prelude::*;

use pyo3::wrap_pymodule;

mod section_models;
mod result_structs;
mod line_force_model;
mod lifting_line;
mod smoothing;
mod wind;

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<result_structs::SectionalForcesInput>()?;
    m.add_class::<result_structs::SectionalForces>()?;
    m.add_class::<result_structs::IntegratedValues>()?;
    m.add_class::<result_structs::SimulationResult>()?;
    
    m.add_wrapped(wrap_pymodule!(section_models::section_models))?;
    m.add_wrapped(wrap_pymodule!(line_force_model::line_force_model))?;
    m.add_wrapped(wrap_pymodule!(lifting_line::lifting_line))?;
    m.add_wrapped(wrap_pymodule!(smoothing::smoothing))?;
    m.add_wrapped(wrap_pymodule!(wind::wind))?;
    
    Ok(())
}
