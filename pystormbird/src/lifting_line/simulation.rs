// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a dynamic simulation using a lifting line model.

use pyo3::prelude::*;

use stormbird::lifting_line::simulation::Simulation as SimulationRust;
use math_utils::spatial_vector::SpatialVector as SpatialVectorRust;

use crate::spatial_vector::SpatialVector;
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
            initialization_velocity
        )
    )]
    pub fn new(
        setup_string: String, 
        initial_time_step: f64, 
        initialization_velocity: SpatialVector
    ) -> Self {
        Self {
            data: SimulationRust::new_from_string(
                &setup_string, 
                initial_time_step, 
                initialization_velocity.data
            ).unwrap()
        }
    }

    pub fn set_translation(&mut self, translation: SpatialVector) {
        self.data.line_force_model.translation = translation.data;
    }

    pub fn set_rotation(&mut self, rotation: SpatialVector) {
        self.data.line_force_model.rotation = rotation.data;
    }

    pub fn set_local_wing_angles(&mut self, local_wing_angles: Vec<f64>) {
        assert!(
            local_wing_angles.len() == self.data.line_force_model.local_wing_angles.len(),
            "The number of wing angles must match the number of wings in the simulation."
        );

        self.data.line_force_model.local_wing_angles = local_wing_angles;
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

        let rust_freestream_velocity: Vec<SpatialVectorRust<3>> = freestream_velocity.iter().map(
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
    pub fn induced_velocities(&self, points: Vec<SpatialVector>, off_body: bool) -> Vec<SpatialVector> {
        let rust_points: Vec<SpatialVectorRust<3>> = points.iter().map(
            |v| v.data
        ).collect();

        let rust_induced_velocities = self.data.induced_velocities(&rust_points, off_body);

        rust_induced_velocities.iter().map(
            |v| SpatialVector::new(v[0], v[1], v[2])
        ).collect()
    }

    #[pyo3(signature=(
        *,
        time, 
        time_step,
        freestream_velocity,
    ))]
    pub fn initialize_with_elliptic_distribution(
        &mut self, 
        time: f64, 
        time_step: f64,
        freestream_velocity: Vec<SpatialVector>,
    ) {

        let rust_freestream_velocity: Vec<SpatialVectorRust<3>> = freestream_velocity.iter().map(
            |v| v.data
        ).collect();

        
        self.data.initialize_with_elliptic_distribution(
            time, 
            time_step,
            &rust_freestream_velocity
        );
    }
}