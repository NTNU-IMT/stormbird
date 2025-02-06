use pyo3::prelude::*;

use math_utils::spatial_vector::SpatialVector as SpatialVectorRust;

#[pyclass]
#[derive(Clone, Debug)]
pub struct SpatialVector{
    data: SpatialVectorRust<3>
}

#[pymethods]
impl SpatialVector  {
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        SpatialVector {
            data: SpatialVectorRust([x, y, z])
        }
    }

    pub fn rotate_around_axis(&self, angle: f64, axis: Self) -> Self {
        SpatialVector {
            data: self.data.rotate_around_axis(angle, axis.data)
        }
    }

    pub fn dot(&self, other: SpatialVector) -> f64 {
        self.data.dot(other.data)
    }

    pub fn cross(&self, other: SpatialVector) -> Self {
        SpatialVector {
            data: self.data.cross(other.data)
        }
    }

    #[getter]
    fn x(&self) -> f64 {
        self.data[0]
    }

    #[getter]
    fn y(&self) -> f64 {
        self.data[1]
    }

    #[getter]
    fn z(&self) -> f64 {
        self.data[2]
    }

    #[setter]
    fn set_x(&mut self, x: f64) {
        self.data[0] = x;
    }

    #[setter]
    fn set_y(&mut self, y: f64) {
        self.data[1] = y;
    }

    #[setter]
    fn set_z(&mut self, z: f64) {
        self.data[2] = z;
    }

    fn __add__(&self, other: Self) -> Self {
        SpatialVector {
            data: self.data + other.data
        }
    }

    fn __mul__(&self, other: f64) -> Self {
        SpatialVector {
            data: self.data * other
        }
    }
}