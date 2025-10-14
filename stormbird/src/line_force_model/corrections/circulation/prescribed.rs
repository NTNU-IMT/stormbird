// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::INFINITY;
use stormath::optimize::{
    curve_fit::CurveFit,
    bounded_variable::BoundedVariable,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrescribedCirculation {
    #[serde(default)]
    pub shape: PrescribedCirculationShape,
    #[serde(default)]
    pub curve_fit_shape_parameters: bool,
}

impl Default for PrescribedCirculation {
    fn default() -> Self {
        PrescribedCirculation {
            shape: PrescribedCirculationShape::default(),
            curve_fit_shape_parameters: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parametric model used to calculate the circulation distribution 
pub struct PrescribedCirculationShape {
    #[serde(default = "PrescribedCirculationShape::default_inner_power")]
    pub inner_power: Float,
    #[serde(default = "PrescribedCirculationShape::default_outer_power")]
    pub outer_power: Float,
}

impl Default for PrescribedCirculationShape {
    /// Default corresponds to an elliptical circulation distribution
    fn default() -> Self {
        PrescribedCirculationShape {
            inner_power: PrescribedCirculationShape::default_inner_power(),
            outer_power: PrescribedCirculationShape::default_outer_power(),
        }
    }
}

impl PrescribedCirculationShape {
    pub fn default_inner_power() -> Float {2.0}
    pub fn default_outer_power() -> Float {0.5}

    pub fn value(&self, s: Float) -> Float {
        let base = 1.0 - (2.0 * s.abs()).powf(self.inner_power);

        if base <= 0.0 {
            return 0.0;
        }

        base.powf(self.outer_power)
    }

    /// Returns the circulation distribution based on the relative span distance
    pub fn get_values(&self, s: &[Float]) -> Vec<Float> {
        s.iter().map(|&x| self.value(x)).collect()
    }

    pub fn as_params_vector(&self) -> Vec<Float> {
        vec![
            1.0,
            self.inner_power,
            self.outer_power
        ]
    }

    pub fn from_params_vector(params: &[Float]) -> Self {
        if params.len() != 3 {
            panic!("PrescribedCirculationShape::from_params_vector expects a vector of length 3");
        }

        PrescribedCirculationShape {
            inner_power: params[1],
            outer_power: params[2],
        }
    }

    pub fn function_to_curve_fit(s: Float, params: &[Float]) -> Float {
        let shape = PrescribedCirculationShape::from_params_vector(params);
        
        params[0] * shape.value(s)
    }

    pub fn from_curve_fit(x_data: &[Float], y_data: &[Float], initial_params: &[Float]) -> Self {       
        let y_data_abs: Vec<Float> = y_data.iter().map(|&y| y.abs()).collect();
        
        let curve_fitter = CurveFit{
            function: PrescribedCirculationShape::function_to_curve_fit,
            max_iterations: 10,
            param_bounds: Some(vec![
                BoundedVariable { min: 1e-6, max: INFINITY }, // scale factor
                BoundedVariable { min: 2.0, max: 4.0 }, // inner power
                BoundedVariable { min: 0.1, max: 1.0 }, // outer power
            ]),
            ..Default::default()
        };

        let resulting_params = curve_fitter.fit_parameters(
            x_data, 
            &y_data_abs, 
            initial_params
        );

        Self::from_params_vector(&resulting_params)
    }
}