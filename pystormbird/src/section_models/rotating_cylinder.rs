// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormbird::section_models::rotating_cylinder::RotatingCylinder as RotatingCylinderRust;
use stormbird::section_models::SectionModel as SectionModelRust;

use super::SectionModel;

#[pyclass]
#[derive(Clone)]
pub struct RotatingCylinder {
    pub data: RotatingCylinderRust
}

#[pymethods]
impl RotatingCylinder {
    #[new]
    #[pyo3(
        signature = (
            revolutions_per_second,
            spin_ratio_data = RotatingCylinderRust::default_spin_ratio_data(),
            cl_data         = RotatingCylinderRust::default_cl_data(),
            cd_data         = RotatingCylinderRust::default_cd_data(),
            wake_angle_data = RotatingCylinderRust::default_wake_angle_data(),
        )
    )]
    pub fn new(
        revolutions_per_second: f64,
        spin_ratio_data: Vec<f64>,
        cl_data: Vec<f64>,
        cd_data: Vec<f64>,
        wake_angle_data: Vec<f64>
    ) -> Self {
        Self {
            data: RotatingCylinderRust {
                revolutions_per_second,
                spin_ratio_data,
                cl_data,
                cd_data,
                wake_angle_data,
                ..Default::default()
            }
        }
    }

    pub fn spin_ratio(&self, diameter: f64, velocity: f64) -> f64 {
        self.data.spin_ratio(diameter, velocity)
    }

    pub fn lift_coefficient_from_spin_ratio(&self, spin_ratio: f64) -> f64 {
        self.data.lift_coefficient_from_spin_ratio(spin_ratio)
    }

    pub fn lift_coefficient(&self, diameter: f64, velocity: f64) -> f64 {
        self.data.lift_coefficient(diameter, velocity)
    }

    pub fn drag_coefficient(&self, diameter: f64, velocity: f64) -> f64 {
        self.data.drag_coefficient(diameter, velocity)
    }

    pub fn drag_coefficient_from_spin_ratio(&self, spin_ratio: f64) -> f64 {
        self.data.drag_coefficient_from_spin_ratio(spin_ratio)
    }

    pub fn as_section_model(&self) -> SectionModel {
        SectionModel {
            data: SectionModelRust::RotatingCylinder(self.data.clone())
        }
    }
}