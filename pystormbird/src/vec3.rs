// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormbird::vec3::Vec3 as Vec3Rust;

#[pyclass]
#[derive(Clone)]
pub struct Vec3 {
    pub data: Vec3Rust
}

impl From<Vec3Rust> for Vec3 {
    fn from(v: Vec3Rust) -> Self {
        Self {
            data: v
        }
    }
}

#[pymethods]
impl Vec3 {
    #[new]
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            data: Vec3Rust::new(x, y, z)
        }
    }

    #[getter]
    pub fn x(&self) -> f64 {
        self.data.x
    }

    #[getter]
    pub fn y(&self) -> f64 {
        self.data.y
    }

    #[getter]
    pub fn z(&self) -> f64 {
        self.data.z
    }

    pub fn rotate(&self, rotation: Vec3) -> Self {
        Self {
            data: self.data.rotate(rotation.data)
        }
    }

    pub fn dot(&self, other: Vec3) -> f64 {
        self.data.dot(other.data)
    }

    #[getter]
    pub fn length(&self) -> f64 {
        self.data.length()
    }
}