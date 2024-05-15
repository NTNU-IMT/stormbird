// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

pub mod span_line;
pub mod builder;

use crate::vec3::Vec3;

use stormbird::line_force_model::LineForceModel as LineForceModelRust;

#[pyclass]
#[derive(Clone)]
pub struct LineForceModel {
    pub data: LineForceModelRust,
}

#[pymethods]
impl LineForceModel {
    #[getter]
    pub fn ctrl_points(&self) -> Vec<Vec3> {
        self.data.ctrl_points().iter().map(|v| Vec3::from(v.clone())).collect()
    }
}

#[pymodule]
pub fn line_force_model(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<span_line::SpanLine>()?;
    m.add_class::<builder::WingBuilder>()?;
    m.add_class::<builder::LineForceModelBuilder>()?;
    m.add_class::<LineForceModel>()?;
    Ok(())
}