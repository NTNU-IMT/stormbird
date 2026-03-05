// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub mod environment;
pub mod wind_condition;
pub mod gust_spectrums;

use pyo3::prelude::*;

#[pymodule]
pub fn wind(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<environment::WindEnvironment>()?;
    m.add_class::<wind_condition::WindCondition>()?;
    m.add_class::<gust_spectrums::DiscretizedSpectrum>()?;
    
    Ok(())
}
