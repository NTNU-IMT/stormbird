// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

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
    pub fn new(input_string: String) -> Self {
        Self {
            data: RotatingCylinderRust::new_from_string(&input_string)
        }
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

    pub fn as_section_model(&self) -> SectionModel {
        SectionModel {
            data: SectionModelRust::RotatingCylinder(self.data.clone())
        }
    }
}
