// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

pub mod singularity_elements;
pub mod simulation;

#[pymodule]
pub fn lifting_line(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<singularity_elements::PotentialTheoryModel>()?;
    m.add_class::<singularity_elements::viscous_core_length::ViscousCoreLength>()?;
    m.add_class::<singularity_elements::symmetry_condition::SymmetryCondition>()?;
    m.add_class::<simulation::Simulation>()?;
    
    Ok(())
}