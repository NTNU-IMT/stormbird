// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use stormath::smoothing::{
    gaussian::GaussianSmoothing as GaussianSmoothingRust,
    end_condition::EndCondition,
};

#[pyclass]
pub struct GaussianSmoothing {
    model: GaussianSmoothingRust<f64>
}

#[pymethods]
impl GaussianSmoothing {
    #[new]
    pub fn new(
        smoothing_length: f64
    ) -> Self {
        let end_conditions = [EndCondition::<f64>::Zero, EndCondition::<f64>::Zero];
        
        Self {
            model: GaussianSmoothingRust {
                smoothing_length,
                end_conditions,
                number_of_end_insertions: None,
                delta_x_factor_end_insertions: 1.0,
            }
        }
    }
    
    pub fn apply_smoothing(&self, x: Vec<f64>, y: Vec<f64>) -> Vec<f64> {
        self.model.apply_smoothing(&x, &y)
    }
    
    pub fn apply_smoothing_with_varying_smoothing_weight(
        &self,
        x: Vec<f64>, 
        y: Vec<f64>,
        smoothing_weight: Vec<f64>
    ) -> Vec<f64> {
        self.model.apply_smoothing_with_varying_smoothing_weight(&x, &y, &smoothing_weight)
    }
}

#[pymodule]
pub fn smoothing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GaussianSmoothing>()?;
    
    Ok(())
}