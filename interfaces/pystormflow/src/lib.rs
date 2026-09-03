// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! A Python interface to run CFD simulations with Stormflow

use pyo3::prelude::*;

pub mod simulation;
pub mod result_structs;

/// A Python module implemented in Rust.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<result_structs::SectionalForcesInput>()?;
    m.add_class::<result_structs::SectionalForces>()?;
    m.add_class::<result_structs::IntegratedValues>()?;
    m.add_class::<result_structs::SimulationResult>()?;

    m.add_class::<simulation::Simulation>()?;
       
    Ok(())
}
