// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a dynamic simulation using a lifting line model.

use pyo3::prelude::*;
use pyo3::types::PyBytes;

use stormbird::lifting_line::simulation::Simulation as SimulationRust;
use stormath::spatial_vector::SpatialVector;

use crate::result_structs::SimulationResult;

use crate::line_force_model::LineForceModel;

#[pyclass]
pub struct Simulation {
    data: SimulationRust
}

#[pymethods]
impl Simulation {
    #[new]
    pub fn new(setup_string: String) -> Self {
        Self {
            data: SimulationRust::new_from_string(&setup_string).unwrap()
        }
    }
    
    #[getter]
    pub fn line_force_model(&self) -> LineForceModel {
        LineForceModel { data: self.data.line_force_model.clone() }
    }

    pub fn set_translation_and_rotation_with_finite_difference_for_the_velocity(
        &mut self,
        time_step: f64,
        translation: [f64; 3],
        rotation: [f64; 3]
    ) {
        self.data.line_force_model.set_translation_and_rotation_with_finite_difference_for_the_velocity(
            time_step,
            SpatialVector::from(translation),
            SpatialVector::from(rotation)
        )
    }

    pub fn set_translation_with_velocity_using_finite_difference(
        &mut self, translation: [f64; 3], time_step: f64
    ) {
        let translation_vector = SpatialVector::new(
            translation[0], translation[1], translation[2]
        );

        self.data.line_force_model.rigid_body_motion.update_translation_with_velocity_using_finite_difference(
            translation_vector,
            time_step
        );

        self.data.line_force_model.update_global_data_representations();
    }

    pub fn set_rotation_with_velocity_using_finite_difference(
        &mut self, rotation: [f64; 3], time_step: f64
    ) {
        let rotation_vector = SpatialVector::from(rotation);

        self.data.line_force_model.rigid_body_motion.update_rotation_with_velocity_using_finite_difference(
            rotation_vector,
            time_step
        );

        self.data.line_force_model.update_global_data_representations();
    }

    pub fn set_translation_only(&mut self, translation: [f64; 3]) {
        self.data.line_force_model.set_translation_only(SpatialVector::from(translation))
    }

    pub fn set_rotation_only( &mut self, rotation: [f64; 3]) {
        self.data.line_force_model.set_rotation_only(SpatialVector::from(rotation))
    }

    pub fn set_velocity_linear(&mut self, linear_velocity: [f64; 3]) {
        self.data.line_force_model.rigid_body_motion.velocity_linear = SpatialVector::from(linear_velocity);
    }

    pub fn set_velocity_angular(&mut self, angular_velocity: [f64; 3]) {
        self.data.line_force_model.rigid_body_motion.velocity_angular = SpatialVector::from(angular_velocity);
    }
    
    pub fn reset_previous_circulation_strength(&mut self) {
        let nr_sections = self.data.line_force_model.nr_span_lines();
        
        self.data.previous_circulation_strength = vec![0.0; nr_sections];
    }

    pub fn set_local_wing_angles(&mut self, local_wing_angles: Vec<f64>) {
        assert!(
            local_wing_angles.len() == self.data.line_force_model.local_wing_angles.len(),
            "The number of wing angles must match the number of wings in the simulation."
        );

        self.data.line_force_model.local_wing_angles = local_wing_angles;

        self.data.line_force_model.update_global_data_representations();
    }

    pub fn set_section_models_internal_state(&mut self, internal_state: Vec<f64>) {
        self.data.line_force_model.set_section_models_internal_state(&internal_state);
    }

    pub fn get_freestream_velocity_points(&self) -> Vec<[f64; 3]> {
        let rust_vec = self.data.get_freestream_velocity_points();

        rust_vec.iter().map(
            |v| [v[0], v[1], v[2]]
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
        freestream_velocity: Vec<[f64; 3]>,
    ) -> SimulationResult {

        let rust_freestream_velocity: Vec<SpatialVector> = freestream_velocity.iter().map(
            |v| SpatialVector::from(*v)
        ).collect();

        SimulationResult {
            data: self.data.do_step(
                time,
                time_step,
                &rust_freestream_velocity
            )
        }
    }

    /// Return the current dynamic wake as VTK PolyData bytes, ready to be fed
    /// into pyvista for visualization, without touching the disk.
    ///
    /// Returns `None` when the simulation uses a static (quasi-steady) wake.
    ///
    /// The `binary` switch selects between the ASCII and BINARY variants of the
    /// VTK legacy format. The returned `bytes` can be read straight into a mesh,
    /// e.g. with pyvista:
    ///
    ///     import vtk
    ///     import pyvista as pv
    ///
    ///     data = simulation.dynamic_wake_as_vtk(binary=True)
    ///     if data is not None:
    ///         reader = vtk.vtkPolyDataReader()
    ///         reader.ReadFromInputStringOn()
    ///         reader.SetBinaryInputString(data, len(data))  # use SetInputString for ASCII
    ///         reader.ReadAllScalarsOn()
    ///         reader.Update()
    ///         mesh = pv.wrap(reader.GetOutput())
    #[pyo3(signature=(binary=true))]
    pub fn dynamic_wake_as_vtk<'py>(
        &self,
        py: Python<'py>,
        binary: bool,
    ) -> Option<Bound<'py, PyBytes>> {
        self.data.dynamic_wake_as_vtk(binary).map(|bytes| PyBytes::new(py, &bytes))
    }

    #[pyo3(signature=(
        points
    ))]
    pub fn induced_velocities(&self, points: Vec<[f64; 3]>) -> Vec<[f64; 3]> {
        let rust_points: Vec<SpatialVector> = points.iter().map(
            |v| SpatialVector::from(*v)
        ).collect();

        let rust_induced_velocities = self.data.induced_velocities(&rust_points);

        rust_induced_velocities.iter().map(
            |v| [v[0], v[1], v[2]]
        ).collect()
    }
}
