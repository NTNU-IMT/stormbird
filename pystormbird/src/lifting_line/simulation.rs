// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a dynamic simulation using a lifting line model.

use pyo3::prelude::*;

use stormbird::lifting_line::simulation::Simulation as SimulationRust;
use stormath::spatial_vector::SpatialVector as SpatialVectorRust;

use crate::spatial_vector::SpatialVector;
use crate::result_structs::SimulationResult;

#[pyclass]
pub struct Simulation {
    data: SimulationRust
}

#[pymethods]
impl Simulation {
    #[new]
    pub fn new(
        setup_string: String
    ) -> Self {
        Self {
            data: SimulationRust::new_from_string(
                &setup_string
            ).unwrap()
        }
    }

    pub fn set_translation_with_velocity_using_finite_difference(&mut self, translation: SpatialVector, time_step: f64) {
        self.data.line_force_model.rigid_body_motion.update_translation_with_velocity_using_finite_difference(
            translation.data, 
            time_step
        );
    }

    pub fn set_rotation_with_velocity_using_finite_difference(&mut self, rotation: SpatialVector, time_step: f64) {
        self.data.line_force_model.rigid_body_motion.update_rotation_with_velocity_using_finite_difference(
            rotation.data, 
            time_step
        );
    }

    pub fn set_translation_without_velocity(&mut self, translation: SpatialVector) {
        self.data.line_force_model.rigid_body_motion.translation = translation.data;
    }

    pub fn set_rotation_without_velocity( &mut self, rotation: SpatialVector) {
        self.data.line_force_model.rigid_body_motion.rotation = rotation.data;
    }

    pub fn set_local_wing_angles(&mut self, local_wing_angles: Vec<f64>) {
        assert!(
            local_wing_angles.len() == self.data.line_force_model.local_wing_angles.len(),
            "The number of wing angles must match the number of wings in the simulation."
        );

        self.data.line_force_model.local_wing_angles = local_wing_angles;
    }

    pub fn set_section_models_internal_state(&mut self, internal_state: Vec<f64>) {
        self.data.line_force_model.set_section_models_internal_state(&internal_state);
    }

    pub fn get_freestream_velocity_points(&self) -> Vec<SpatialVector> {
        let rust_vec = self.data.get_freestream_velocity_points();

        rust_vec.iter().map(
            |v| SpatialVector::new(v[0], v[1], v[2])
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
        freestream_velocity: Vec<SpatialVector>,
    ) -> SimulationResult {

        let rust_freestream_velocity: Vec<SpatialVectorRust> = freestream_velocity.iter().map(
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
        points
    ))]
    pub fn induced_velocities(&self, points: Vec<SpatialVector>) -> Vec<SpatialVector> {
        let rust_points: Vec<SpatialVectorRust> = points.iter().map(
            |v| v.data
        ).collect();

        let rust_induced_velocities = self.data.induced_velocities(&rust_points);

        rust_induced_velocities.iter().map(
            |v| SpatialVector::new(v[0], v[1], v[2])
        ).collect()
    }
}