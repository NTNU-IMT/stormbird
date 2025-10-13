// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

pub mod singularity_elements;
pub mod simulation;
pub mod complete_sail_model;

#[pymodule]
pub fn lifting_line(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<singularity_elements::PotentialTheorySettings>()?;
    m.add_class::<singularity_elements::symmetry_condition::SymmetryCondition>()?;
    m.add_class::<simulation::Simulation>()?;
    m.add_class::<complete_sail_model::CompleteSailModel>()?;
    
    Ok(())
}