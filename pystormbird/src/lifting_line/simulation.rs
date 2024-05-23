// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a dynamic simulation using a lifting line model.

use pyo3::prelude::*;

use stormbird::lifting_line::simulation::Simulation as SimulationRust;
use stormbird::vec3::Vec3 as Vec3Rust;

use crate::vec3::Vec3;
use crate::result_structs::SimulationResult;

#[pyclass]
pub struct Simulation {
    data: SimulationRust
}

#[pymethods]
impl Simulation {
    #[new]
    #[pyo3(
        signature=(
            *,
            setup_string, 
            initial_time_step,
            wake_initial_velocity
        )
    )]
    pub fn new(
        setup_string: String, 
        initial_time_step: f64, 
        wake_initial_velocity: Vec3
    ) -> Self {
        Self {
            data: SimulationRust::new_from_string(
                &setup_string, 
                initial_time_step, 
                wake_initial_velocity.data
            ).unwrap()
        }
    }

    pub fn set_translation(&mut self, translation: Vec3) {
        self.data.line_force_model.translation = translation.data;
    }

    pub fn set_rotation(&mut self, rotation: Vec3) {
        self.data.line_force_model.rotation = rotation.data;
    }

    pub fn set_local_wing_angles(&mut self, local_wing_angles: Vec<f64>) {
        assert!(
            local_wing_angles.len() == self.data.line_force_model.local_wing_angles.len(),
            "The number of wing angles must match the number of wings in the simulation."
        );

        self.data.line_force_model.local_wing_angles = local_wing_angles;
    }

    pub fn get_freestream_velocity_points(&self) -> Vec<Vec3> {
        let rust_vec = self.data.get_freestream_velocity_points();

        rust_vec.iter().map(
            |v| Vec3::new(v.x, v.y, v.z)
        ).collect()
    }

    #[pyo3(signature=(
        *,
        time, 
        time_step,
        freestream_velocity,
    ))]
    pub fn do_step(
        &mut self, 
        time: f64, 
        time_step: f64,
        freestream_velocity: Vec<Vec3>,
    ) -> SimulationResult {

        let rust_freestream_velocity: Vec<Vec3Rust> = freestream_velocity.iter().map(
            |v| v.data
        ).collect();

        SimulationResult {
            data: self.data.do_step(
                time, 
                time_step,
                &rust_freestream_velocity
            )
        }
    }

    #[pyo3(signature=(
        points,
        off_body = true
    ))]
    pub fn induced_velocities(&self, points: Vec<Vec3>, off_body: bool) -> Vec<Vec3> {
        let rust_points: Vec<Vec3Rust> = points.iter().map(
            |v| v.data
        ).collect();

        let rust_induced_velocities = self.data.induced_velocities(&rust_points, off_body);

        rust_induced_velocities.iter().map(
            |v| Vec3::new(v.x, v.y, v.z)
        ).collect()
    }
}