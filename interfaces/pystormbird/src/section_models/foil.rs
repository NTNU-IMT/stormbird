// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pythonize::pythonize;

use stormbird::section_models::foil::Foil as FoilRust;

use stormbird::section_models::SectionModel as SectionModelRust;
use super::SectionModel;

#[pyclass]
#[derive(Clone)]
pub struct Foil {
    pub data: FoilRust
}

#[pymethods]
impl Foil {
    #[new]
    pub fn new(input_string: String) -> Self {
        Self {
            data: FoilRust::new_from_string(&input_string).unwrap()
        }
    }

    pub fn lift_coefficient(&self, angle_of_attack: f64) -> f64 {
        self.data.lift_coefficient(angle_of_attack)
    }

    pub fn drag_coefficient(&self, angle_of_attack: f64) -> f64 {
        self.data.drag_coefficient(angle_of_attack)
    }

    pub fn as_section_model(&self) -> SectionModel {
        SectionModel {
            data: SectionModelRust::Foil(self.data.clone())
        }
    }

    #[getter]
    pub fn get_cl_zero_angle(&self) -> f64 {
        self.data.cl_zero_angle
    }

    #[setter]
    pub fn set_cl_zero_angle(&mut self, value: f64) {
        self.data.cl_zero_angle = value;
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
    pub fn get_cl_high_order_factor_positive(&self) -> f64 {
        self.data.cl_high_order_factor_positive
    }

    #[setter]
    pub fn set_cl_high_order_factor_positive(&mut self, value: f64) {
        self.data.cl_high_order_factor_positive= value;
    }
    
    #[getter]
    pub fn get_cl_high_order_factor_negative(&self) -> f64 {
        self.data.cl_high_order_factor_negative
    }

    #[setter]
    pub fn set_cl_high_order_factor_negative(&mut self, value: f64) {
        self.data.cl_high_order_factor_negative = value;
    }

    #[getter]
    pub fn get_cl_high_order_power(&self) -> f64 {
        self.data.cl_high_order_power
    }

    #[setter]
    pub fn set_cl_high_order_power(&mut self, value: f64) {
        self.data.cl_high_order_power = value;
    }

    #[getter]
    pub fn get_cl_max_after_stall(&self) -> f64 {
        self.data.cl_max_after_stall
    }

    #[setter]
    pub fn set_cl_max_after_stall(&mut self, value: f64) {
        self.data.cl_max_after_stall = value;
    }

    #[getter]
    pub fn get_cd_min(&self) -> f64 {
        self.data.cd_min
    }

    #[setter]
    pub fn set_cd_min(&mut self, value: f64) {
        self.data.cd_min = value;
    }

    #[getter]
    pub fn get_angle_cd_min(&self) -> f64 {
        self.data.angle_cd_min
    }

    #[setter]
    pub fn set_angle_cd_min(&mut self, value: f64) {
        self.data.angle_cd_min = value;
    }

    #[getter]
    pub fn get_cd_second_order_factor(&self) -> f64 {
        self.data.cd_second_order_factor
    }

    #[setter]
    pub fn set_cd_second_order_factor(&mut self, value: f64) {
        self.data.cd_second_order_factor = value;
    }

    #[getter]
    pub fn get_cd_max_after_stall(&self) -> f64 {
        self.data.cd_max_after_stall
    }

    #[setter]
    pub fn set_cd_max_after_stall(&mut self, value: f64) {
        self.data.cd_max_after_stall = value;
    }

    #[getter]
    pub fn get_cd_power_after_stall(&self) -> f64 {
        self.data.cd_power_after_stall
    }

    #[setter]
    pub fn set_cd_power_after_stall(&mut self, value: f64) {
        self.data.cd_power_after_stall = value;
    }

    #[getter]
    pub fn get_mean_positive_stall_angle(&self) -> f64 {
        self.data.mean_positive_stall_angle
    }

    #[setter]
    pub fn set_mean_positive_stall_angle(&mut self, value: f64) {
        self.data.mean_positive_stall_angle = value;
    }

    #[getter]
    pub fn get_mean_negative_stall_angle(&self) -> f64 {
        self.data.mean_negative_stall_angle
    }

    #[setter]
    pub fn set_mean_negative_stall_angle(&mut self, value: f64) {
        self.data.mean_negative_stall_angle = value;
    }

    #[getter]
    pub fn get_stall_range(&self) -> f64 {
        self.data.stall_range
    }

    #[setter]
    pub fn set_stall_range(&mut self, value: f64) {
        self.data.stall_range = value;
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