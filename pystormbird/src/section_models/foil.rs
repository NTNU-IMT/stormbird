// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

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
    #[pyo3(
        signature = (
            *,
            cl_zero_angle          = 0.0,
            cl_initial_slope       = FoilRust::default_cl_initial_slope(),
            cl_high_order_factor   = 0.0, 
            cl_high_order_power    = 0.0,
            cl_max_after_stall     = 1.0, 
            cd_zero_angle          = 0.0, 
            cd_second_order_factor = 0.0, 
            cd_max_after_stall     = 1.0, 
            cd_power_after_stall   = FoilRust::default_cd_power_after_stall(), 
            mean_stall_angle       = FoilRust::default_mean_stall_angle(), 
            stall_range            = FoilRust::default_stall_range()
        )
    )]
    pub fn new(
        cl_zero_angle: f64,
        cl_initial_slope: f64,
        cl_high_order_factor: f64,
        cl_high_order_power: f64,
        cl_max_after_stall: f64,
        cd_zero_angle: f64,
        cd_second_order_factor: f64,
        cd_max_after_stall: f64,
        cd_power_after_stall: f64,
        mean_stall_angle: f64,
        stall_range: f64,
    ) -> Self {
        Self {
            data: FoilRust {
                cl_zero_angle,
                cl_initial_slope,
                cl_high_order_factor,
                cl_high_order_power,
                cl_max_after_stall,
                cd_zero_angle,
                cd_second_order_factor,
                cd_max_after_stall,
                cd_power_after_stall,
                mean_stall_angle,
                stall_range,
                ..Default::default()
            }
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
    pub fn cl_zero_angle(&self) -> f64 {
        self.data.cl_zero_angle
    }

    #[getter]
    pub fn cl_initial_slope(&self) -> f64 {
        self.data.cl_initial_slope
    }

    #[getter]
    pub fn cl_high_order_factor(&self) -> f64 {
        self.data.cl_high_order_factor
    }

    #[getter]
    pub fn cl_high_order_power(&self) -> f64 {
        self.data.cl_high_order_power
    }

    #[getter]
    pub fn cl_max_after_stall(&self) -> f64 {
        self.data.cl_max_after_stall
    }

    #[getter]
    pub fn cd_zero_angle(&self) -> f64 {
        self.data.cd_zero_angle
    }

    #[getter]
    pub fn cd_second_order_factor(&self) -> f64 {
        self.data.cd_second_order_factor
    }

    #[getter]
    pub fn cd_max_after_stall(&self) -> f64 {
        self.data.cd_max_after_stall
    }

    #[getter]
    pub fn cd_power_after_stall(&self) -> f64 {
        self.data.cd_power_after_stall
    }

    #[getter]
    pub fn mean_stall_angle(&self) -> f64 {
        self.data.mean_stall_angle
    }

    #[getter]
    pub fn stall_range(&self) -> f64 {
        self.data.stall_range
    }
}