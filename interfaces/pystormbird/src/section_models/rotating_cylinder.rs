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
    pub fn new(input_string: String) -> Self {
        Self {
            data: RotatingCylinderRust::new_from_string(&input_string)
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