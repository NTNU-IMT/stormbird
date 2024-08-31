// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormbird::line_force_model::span_line::SpanLine as SpanLineRust;

use crate::spatial_vector::SpatialVector;

#[pyclass]
pub struct SpanLine {
    pub data: SpanLineRust
}

#[pymethods]
impl SpanLine {
    #[new]
    #[pyo3(signature = (start_point, end_point))]
    fn new(start_point: SpatialVector, end_point: SpatialVector) -> Self {
        Self {
            data: SpanLineRust {
                start_point: start_point.data,
                end_point: end_point.data,
            }
        }
    }

    #[getter]
    fn start_point(&self) -> SpatialVector {
        SpatialVector {
            data: self.data.start_point
        }
    }

    #[getter]
    fn end_point(&self) -> SpatialVector {
        SpatialVector {
            data: self.data.end_point
        }
    }

    fn relative_vector(&self) -> SpatialVector {
        SpatialVector {
            data: self.data.relative_vector()
        }
    }

    fn length(&self) -> f64 {
        self.data.length()
    }

    fn direction(&self) -> SpatialVector {
        SpatialVector {
            data: self.data.direction()
        }
    }

    /// Return the control point of the line segment, which corresponds to the average point along 
    /// the line segment. 
    fn ctrl_point(&self) -> SpatialVector {
        SpatialVector {
            data: self.data.ctrl_point()
        }
    }

    fn distance(&self, point: SpatialVector) -> f64 {
        self.data.distance(point.data)
    }

}