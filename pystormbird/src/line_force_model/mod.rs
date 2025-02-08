// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use stormbird::io_structs::prelude::CoordinateSystem;

pub mod span_line;
pub mod builder;

use crate::spatial_vector::SpatialVector;
use math_utils::spatial_vector::SpatialVector as SpatialVectorRust;

use stormbird::line_force_model::LineForceModel as LineForceModelRust;
use stormbird::line_force_model::builder::LineForceModelBuilder as LineForceModelBuilderRust;

use stormbird::line_force_model::circulation_corrections::smoothing::GaussianSmoothing;

#[pyclass]
#[derive(Clone)]
pub struct LineForceModel {
    pub data: LineForceModelRust,
}

#[pymethods]
impl LineForceModel {
    #[new]
    pub fn new(json_string: String) -> Self {
        let builder = LineForceModelBuilderRust::new_from_string(&json_string).unwrap();

        Self {
            data: builder.build()
        }
    }

    pub fn relative_span_distance(&self) -> Vec<f64> {
        self.data.relative_span_distance()
    }

    pub fn circulation_strength(&self, velocity: Vec<SpatialVector>) -> Vec<f64> {
        let rust_velocity: Vec<SpatialVectorRust<3>> = velocity.iter().map(|v| SpatialVectorRust::from(v.data)).collect();
        self.data.circulation_strength(&rust_velocity, CoordinateSystem::Global)
    }

    pub fn angles_of_attack(&self, velocity: Vec<SpatialVector>) -> Vec<f64> {
        let rust_velocity: Vec<SpatialVectorRust<3>> = velocity.iter().map(|v| SpatialVectorRust::from(v.data)).collect();
        
        self.data.angles_of_attack(&rust_velocity, CoordinateSystem::Global)
    }

    #[pyo3(signature = (
        *,
        noisy_strength, 
        length_factor,
    ))]
    pub fn gaussian_smoothed_strength(
        &self, 
        noisy_strength: Vec<f64>, 
        length_factor: f64, 
    ) -> Vec<f64> {
        let settings = GaussianSmoothing {
            length_factor,
            ..Default::default()
        };

        self.data.gaussian_smoothed_values(&noisy_strength, &settings)
    }

    #[getter]
    pub fn ctrl_points(&self) -> Vec<SpatialVector> {
        self.data.ctrl_points().iter().map(|v| SpatialVector::from(v.clone())).collect()
    }
}

#[pymodule]
pub fn line_force_model(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<span_line::SpanLine>()?;
    m.add_class::<builder::WingBuilder>()?;
    m.add_class::<builder::LineForceModelBuilder>()?;
    m.add_class::<LineForceModel>()?;
    Ok(())
}