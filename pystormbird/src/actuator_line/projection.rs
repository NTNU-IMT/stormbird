// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use pyo3::types::PyType;

use stormbird::actuator_line::projection::Projection as ProjectionRust;

use stormbird::actuator_line::projection::gaussian::Gaussian;
use stormbird::actuator_line::projection::elliptic::Elliptic;
use stormbird::actuator_line::projection::harmonic::Harmonic;
use stormbird::actuator_line::projection::chord_variation_from_data::ChordVariationFromData;

use crate::vec3::Vec3;
use crate::line_force_model::span_line::SpanLine;

#[pyclass]
pub struct Projection {
    pub data: ProjectionRust
}

#[pymethods]
impl Projection {
    #[classmethod]
    pub fn gaussian(_cls: &PyType, chord_factor: f64, thickness_factor: f64) -> Self {
        Self {
            data: ProjectionRust::Gaussian(
                Gaussian {
                    chord_factor, 
                    thickness_factor
                }
            )
        }
    }

    #[classmethod]
    pub fn elliptic(_cls: &PyType, chord_factor: f64, thickness_factor: f64) -> Self {
        Self {
            data: ProjectionRust::Elliptic(
                Elliptic {
                    chord_factor, 
                    thickness_factor
                }
            )
        }
    }

    #[classmethod]
    pub fn harmonic(_cls: &PyType, chord_factor: f64, thickness_factor: f64) -> Self {
        Self {
            data: ProjectionRust::Harmonic(
                Harmonic {
                    chord_factor, 
                    thickness_factor
                }
            )
        }
    }

    #[classmethod]
    #[pyo3(
        signature = (
            *,
            chord_factor,
            thickness_factor,
            chord_factor_data,
            value_data
        )
    )]
    pub fn chord_variation_from_data(_cls: &PyType, chord_factor: f64, thickness_factor: f64, chord_factor_data: Vec<f64>, value_data: Vec<f64>) -> Self {
        Self {
            data: ProjectionRust::ChordVariationFromData(
                ChordVariationFromData {
                    chord_factor, 
                    thickness_factor,
                    chord_factor_data,
                    value_data
                }
            )
        }   
    }

    pub fn projection_value_at_point(&self, point: Vec3, chord_vector: Vec3, span_line: &SpanLine) -> f64 {
        self.data.projection_value_at_point(point.data, chord_vector.data, &span_line.data)
    }
}