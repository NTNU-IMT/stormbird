// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub mod projection;

use pyo3::prelude::*;

use stormbird::actuator_line::ActuatorLine as ActuatorLineRust;
use stormbird::vec3::Vec3 as Vec3Rust;

use crate::vec3::Vec3;
use crate::result_structs::SimulationResult;

#[pyclass]
pub struct ActuatorLine {
    pub data: ActuatorLineRust
}

#[pymethods]
impl ActuatorLine {
    #[new]
    pub fn new(builder_string: &str) -> Self {
        ActuatorLine {
            data: ActuatorLineRust::new_from_string(builder_string)
        }
    }

    pub fn calculate_result(&mut self, velocity: Vec<Vec3>) -> SimulationResult {
        let velocity_rust: Vec<Vec3Rust> = velocity.iter().map(|vec| vec.data).collect();

        SimulationResult {
            data: self.data.calculate_result(&velocity_rust)
        }
    }

    pub fn __str__(&self) -> String {
        format!("{:?}", self.data)
    }

}

#[pymodule]
pub fn actuator_line(_py: Python, m: &PyModule) -> PyResult<()> {    
    m.add_class::<ActuatorLine>()?;
    m.add_class::<projection::Projection>()?;

    Ok(())
}