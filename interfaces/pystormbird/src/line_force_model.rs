// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;
use stormbird::common_utils::prelude::CoordinateSystem;

use stormath::spatial_vector::SpatialVector;
use stormbird::line_force_model::builder::LineForceModelBuilder;

use stormbird::line_force_model::LineForceModel as LineForceModelRust;


#[pyclass]
#[derive(Clone)]
pub struct LineForceModel {
    pub data: LineForceModelRust,
}

#[pymethods]
impl LineForceModel {
    #[new]
    pub fn new(json_string: String) -> Self {
        let builder = LineForceModelBuilder::new_from_string(&json_string).unwrap();

        Self {
            data: builder.build()
        }
    }

    pub fn circulation_strength(&self, angles_of_attack: Vec<f64>, velocity: Vec<[f64; 3]>) -> Vec<f64> {
        let rust_velocity: Vec<SpatialVector> = velocity.iter().map(|v| SpatialVector::from(*v)).collect();
        self.data.circulation_strength(&angles_of_attack, &rust_velocity)
    }

    pub fn angles_of_attack(&self, velocity: Vec<[f64; 3]>) -> Vec<f64> {
        let rust_velocity: Vec<SpatialVector> = velocity.iter().map(|v| SpatialVector::from(*v)).collect();
        
        self.data.angles_of_attack(&rust_velocity, CoordinateSystem::Global)
    }

    #[getter]
    pub fn ctrl_points(&self) -> Vec<[f64; 3]> {
        self.data.ctrl_points_global.iter().map(|v| v.0).collect()
    }

    #[getter]
    pub fn ctrl_point_spanwise_distance_non_dimensional(&self) -> Vec<f64> {
        self.data.ctrl_point_spanwise_distance_non_dimensional.clone()
    }
}

#[pymodule]
pub fn line_force_model(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<LineForceModel>()?;
    Ok(())
}