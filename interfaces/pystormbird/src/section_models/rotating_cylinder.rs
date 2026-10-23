// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pythonize::pythonize;

use stormbird::section_models::rotating_cylinder::RotatingCylinder as RotatingCylinderRust;
use stormbird::section_models::SectionModel as SectionModelRust;

use super::SectionModel;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct RotatingCylinder {
    pub data: RotatingCylinderRust
}

#[pymethods]
impl RotatingCylinder {
    #[new]
    pub fn new(input_string: String) -> PyResult<Self> {
        let data = RotatingCylinderRust::new_from_string(&input_string).map_err(|e| {
            PyValueError::new_err(format!("Failed to parse rotating cylinder from string: {}", e))
        })?;

        Ok(Self { data })
    }

    #[staticmethod]
    pub fn spin_ratio_from_revolutions_per_second(
        revolutions_per_second: f64,
        diameter: f64,
        velocity: f64
    ) -> f64 {
        RotatingCylinderRust::spin_ratio_from_revolutions_per_second(
            revolutions_per_second, diameter, velocity
        )
    }

    #[staticmethod]
    pub fn revolutions_per_second_from_spin_ratio(
        spin_ratio: f64,
        diameter: f64,
        velocity: f64
    ) -> f64 {
        RotatingCylinderRust::revolutions_per_second_from_spin_ratio(
            spin_ratio, diameter, velocity
        )
    }

    pub fn lift_coefficient(&self, spin_ratio: f64) -> f64 {
        self.data.lift_coefficient(spin_ratio)
    }

    pub fn drag_coefficient(&self, spin_ratio: f64) -> f64 {
        self.data.drag_coefficient(spin_ratio)
    }

    pub fn power_coefficient(&self, spin_ratio: f64) -> f64 {
        self.data.power_coefficient(spin_ratio)
    }

    pub fn as_section_model(&self) -> SectionModel {
        SectionModel {
            data: SectionModelRust::RotatingCylinder(self.data.clone())
        }
    }

    #[getter]
    pub fn get_cl_initial_slope(&self) -> f64 {
        self.data.cl_initial_slope
    }

    #[setter]
    pub fn set_cl_initial_slope(&mut self, value: f64) {
        self.data.cl_initial_slope = value;
    }

    #[getter]
    pub fn get_cl_high_spin_ratio_slope(&self) -> f64 {
        self.data.cl_high_spin_ratio_slope
    }

    #[setter]
    pub fn set_cl_high_spin_ratio_slope(&mut self, value: f64) {
        self.data.cl_high_spin_ratio_slope = value;
    }

    #[getter]
    pub fn get_cl_slope_transition_spin_ratio(&self) -> f64 {
        self.data.cl_slope_transition_spin_ratio
    }

    #[setter]
    pub fn set_cl_slope_transition_spin_ratio(&mut self, value: f64) {
        self.data.cl_slope_transition_spin_ratio = value;
    }

    #[getter]
    pub fn get_cl_slope_transition_range(&self) -> f64 {
        self.data.cl_slope_transition_range
    }

    #[setter]
    pub fn set_cl_slope_transition_range(&mut self, value: f64) {
        self.data.cl_slope_transition_range = value;
    }

    #[getter]
    pub fn get_cd_start(&self) -> f64 {
        self.data.cd_start
    }

    #[setter]
    pub fn set_cd_start(&mut self, value: f64) {
        self.data.cd_start = value;
    }

    #[getter]
    pub fn get_cd_end(&self) -> f64 {
        self.data.cd_end
    }

    #[setter]
    pub fn set_cd_end(&mut self, value: f64) {
        self.data.cd_end = value;
    }

    #[getter]
    pub fn get_cd_angle_start(&self) -> f64 {
        self.data.cd_angle_start
    }

    #[setter]
    pub fn set_cd_angle_start(&mut self, value: f64) {
        self.data.cd_angle_start = value;
    }

    #[getter]
    pub fn get_cd_spin_ratio_low_drag(&self) -> f64 {
        self.data.cd_spin_ratio_low_drag
    }

    #[setter]
    pub fn set_cd_spin_ratio_low_drag(&mut self, value: f64) {
        self.data.cd_spin_ratio_low_drag = value;
    }


    #[getter]
    pub fn get_power_coefficient_polynomial(&self) -> Vec<f64> {
        self.data.power_coefficient_polynomial.clone()
    }

    #[setter]
    pub fn set_power_coefficient_polynomial(&mut self, value: Vec<f64>) {
        self.data.power_coefficient_polynomial = value;
    }

    pub fn __str__(&self) -> String {
        self.data.to_string()
    }

    #[getter]
    /// Uses the built in json module to convert the string to a dictionary
    pub fn __dict__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(pythonize(py, &self.data)?)
    }
}
