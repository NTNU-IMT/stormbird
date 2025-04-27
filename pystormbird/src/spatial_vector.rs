// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormath::spatial_vector::SpatialVector as SpatialVectorRust;
use stormath::spatial_vector::transformations::RotationType;

#[pyclass]
#[derive(Clone)]
pub struct SpatialVector {
    pub data: SpatialVectorRust<3>
}

impl From<SpatialVectorRust<3>> for SpatialVector {
    fn from(v: SpatialVectorRust<3>) -> Self {
        Self {
            data: v
        }
    }
}

#[pymethods]
impl SpatialVector {
    #[new]
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            data: SpatialVectorRust([x, y, z])
        }
    }

    #[getter]
    pub fn x(&self) -> f64 {
        self.data[0]
    }

    #[getter]
    pub fn y(&self) -> f64 {
        self.data[1]
    }

    #[getter]
    pub fn z(&self) -> f64 {
        self.data[2]
    }

    pub fn rotate(&self, rotation: SpatialVector) -> Self {
        Self {
            data: self.data.rotate(rotation.data, RotationType::XYZ)
        }
    }

    pub fn dot(&self, other: SpatialVector) -> f64 {
        self.data.dot(other.data)
    }

    #[getter]
    pub fn length(&self) -> f64 {
        self.data.length()
    }
}