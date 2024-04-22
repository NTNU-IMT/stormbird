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
}