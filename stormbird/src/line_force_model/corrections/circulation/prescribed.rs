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
#[derive(Default)]
/// Correction where the circulation is forced to follow a prescribed shape, while the average
/// value is the same as from the lifting line theory
pub struct PrescribedCirculation {
    /// The prescribed shape of the circulation distribution
    #[serde(default)]
    pub shape: PrescribedCirculationShape,
    /// If true, the parameters of the shape are fitted to the raw circulation distribution, rather
    /// than using the given shape directly
    #[serde(default)]
    pub curve_fit_shape_parameters: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parametric model used to calculate the circulation distribution 
pub struct PrescribedCirculationShape {
    /// The power of the non-dimensional span distance, inside the outer power
    #[serde(default = "PrescribedCirculationShape::default_inner_power")]
    pub inner_power: Float,
    /// The power of the whole expression
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
    /// Default inner power, which gives an elliptical shape together with the default outer power
    pub fn default_inner_power() -> Float {2.0}
    /// Default outer power, which gives an elliptical shape together with the default inner power
    pub fn default_outer_power() -> Float {0.5}

    /// Returns the value of the shape at the non-dimensional span location `s`, running from -0.5 to
    /// 0.5: (1 - |2 s|^inner_power)^outer_power
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

    /// Returns the parameters of the shape as a vector, together with a scale factor of 1.0 as the
    /// first value
    pub fn as_params_vector(&self) -> Vec<Float> {
        vec![
            1.0,
            self.inner_power,
            self.outer_power
        ]
    }

    /// Creates a shape from a vector of parameters, where the first value is a scale factor that is
    /// not used. Panics if the vector does not have three values.
    pub fn from_params_vector(params: &[Float]) -> Self {
        if params.len() != 3 {
            panic!("PrescribedCirculationShape::from_params_vector expects a vector of length 3");
        }

        PrescribedCirculationShape {
            inner_power: params[1],
            outer_power: params[2],
        }
    }

    /// The function used when fitting the shape to data, i.e., the shape multiplied by a scale
    /// factor
    pub fn function_to_curve_fit(s: Float, params: &[Float]) -> Float {
        let shape = PrescribedCirculationShape::from_params_vector(params);
        
        params[0] * shape.value(s)
    }

    /// Fits the shape parameters to the absolute values of the given data
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