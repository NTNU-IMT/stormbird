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

#[pymodule]
pub fn section_models(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<SectionModel>()?;
    m.add_class::<foil::Foil>()?;
    m.add_class::<varying_foil::VaryingFoil>()?;
    m.add_class::<rotating_cylinder::RotatingCylinder>()?;
    Ok(())
}