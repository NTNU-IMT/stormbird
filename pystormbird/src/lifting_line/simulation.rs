// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a dynamic simulation using a lifting line model.

use pyo3::prelude::*;
use pyo3::types::PyType;

use stormbird::lifting_line::simulation::Simulation as SimulationRust;
use stormbird::io_structs::input::InputState;

use crate::vec3::Vec3;
use crate::result_structs::SimulationResult;

#[pyclass]
pub struct Simulation {
    data: SimulationRust
}

#[pymethods]
impl Simulation {
    #[classmethod]
    pub fn new_from_string(_cls: &PyType, string: String) -> Self {
        Self {
            data: SimulationRust::new_from_string(&string).unwrap()
        }
    }

    #[pyo3(signature=(
        *,
        time, 
        time_step,
        freestream_velocity,
        translation = Vec3::new(0.0, 0.0, 0.0),
        rotation = Vec3::new(0.0, 0.0, 0.0),
    ))]
    pub fn do_step(
        &mut self, 
        time: f64, 
        time_step: f64,
        freestream_velocity: Vec3,
        translation: Vec3,
        rotation: Vec3,
    ) -> SimulationResult {

        let input_state = InputState {
            freestream_velocity: freestream_velocity.data,
            translation: translation.data,
            rotation: rotation.data,
        };

        SimulationResult {
            data: self.data.do_step(
                time, 
                time_step,
                input_state
            )
        }
    }
}