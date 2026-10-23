// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a dynamic simulation using a lifting line model.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyBytes;

use stormbird::lifting_line::simulation::Simulation as SimulationRust;
use stormath::spatial_vector::SpatialVector;

use crate::result_structs::SimulationResult;

use crate::line_force_model::LineForceModel;
use crate::span_varying_internal_state;

#[pyclass]
pub struct Simulation {
    data: SimulationRust
}

#[pymethods]
impl Simulation {
    #[new]
    pub fn new(setup_string: String) -> PyResult<Self> {
        let data = SimulationRust::new_from_string(&setup_string).map_err(|e| {
            PyValueError::new_err(format!("Failed to create simulation from string: {}", e))
        })?;

        Ok(Self { data })
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

    pub fn internal_state(&self) -> Vec<f64> {
        self.data.line_force_model.internal_state_representative_values()
    }

    pub fn set_internal_state(&mut self, internal_state: Vec<f64>) {
        self.data.line_force_model.set_internal_state(&internal_state);
    }

    /// Sets an internal state for a wing that is linearly interpolated between non-dimensional
    /// span locations, running from -0.5 to 0.5.
    pub fn set_internal_state_linear(
        &mut self,
        wing_index: usize,
        locations: Vec<f64>,
        values: Vec<f64>
    ) -> PyResult<()> {
        span_varying_internal_state::set_internal_state_linear(
            &mut self.data.line_force_model, wing_index, locations, values
        )
    }

    /// Sets an internal state for a wing that is constant within zones along the span. The zones
    /// are defined by their boundaries, given as non-dimensional span locations, with one more
    /// boundary than values.
    pub fn set_internal_state_piecewise_constant(
        &mut self,
        wing_index: usize,
        boundaries: Vec<f64>,
        values: Vec<f64>
    ) -> PyResult<()> {
        span_varying_internal_state::set_internal_state_piecewise_constant(
            &mut self.data.line_force_model, wing_index, boundaries, values
        )
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

    /// Return the current wake as VTK PolyData bytes, ready to be fed into pyvista for
    /// visualization, or written to a file, without any automatic export in the simulation.
    ///
    /// Works for both dynamic and quasi-steady wakes. A dynamic wake is exported as one quad per
    /// panel, while a quasi-steady wake is exported as one polygon per horseshoe vortex. The
    /// strength and viscous core length are included as cell data.
    ///
    /// The `binary` switch selects between the ASCII and BINARY variants of the VTK legacy
    /// format. The returned `bytes` can be read straight into a mesh, e.g. with pyvista:
    ///
    ///     import vtk
    ///     import pyvista as pv
    ///
    ///     data = simulation.wake_as_vtk(binary=True)
    ///     reader = vtk.vtkPolyDataReader()
    ///     reader.ReadFromInputStringOn()
    ///     reader.SetBinaryInputString(data, len(data))  # use SetInputString for ASCII
    ///     reader.ReadAllScalarsOn()
    ///     reader.Update()
    ///     mesh = pv.wrap(reader.GetOutput())
    ///
    /// Or written to a file that can be opened in, for instance, ParaView:
    ///
    ///     with open("wake.vtk", "wb") as f:
    ///         f.write(simulation.wake_as_vtk(binary=True))
    #[pyo3(signature=(binary=true))]
    pub fn wake_as_vtk<'py>(
        &self,
        py: Python<'py>,
        binary: bool,
    ) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data.wake_as_vtk(binary))
    }

    /// Deprecated: use `wake_as_vtk`, which works for both dynamic and quasi-steady wakes. Kept
    /// for backwards compatibility.
    #[pyo3(signature=(binary=true))]
    pub fn dynamic_wake_as_vtk<'py>(
        &self,
        py: Python<'py>,
        binary: bool,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let warning_type = py.get_type::<pyo3::exceptions::PyDeprecationWarning>();

        PyErr::warn(
            py,
            &warning_type,
            c"dynamic_wake_as_vtk is deprecated, use wake_as_vtk instead",
            1
        )?;

        Ok(self.wake_as_vtk(py, binary))
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
