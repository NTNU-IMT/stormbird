use core::f64;

use serde::{Serialize, Deserialize};

use stormath::optimize::{
    curve_fit::CurveFit,
    bounded_variable::BoundedVariable,
};

use stormath::array_generation::linspace;

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
    pub inner_power: f64,
    #[serde(default = "PrescribedCirculationShape::default_outer_power")]
    pub outer_power: f64,
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
    pub fn default_inner_power() -> f64 {2.0}
    pub fn default_outer_power() -> f64 {0.5}

    pub fn value(&self, s: f64) -> f64 {
        if s.abs() < 0.5 {
            (1.0 - (2.0 * s.abs()).powf(self.inner_power))
                .powf(self.outer_power)
        } else {
            0.0
        }
    }

    pub fn mean_value(&self) -> f64 {
        let test_values = linspace(-0.5, 0.5, 100);
        let sum: f64 = test_values.iter().map(|&s| self.value(s)).sum();
        
        sum / test_values.len() as f64  
    }

    /// Returns the circulation distribution based on the relative span distance
    pub fn get_values(&self, s: &[f64]) -> Vec<f64> {
        s.iter().map(|&x| self.value(x)).collect()
    }

    pub fn as_params_vector(&self) -> Vec<f64> {
        vec![
            self.outer_power
        ]
    }

    pub fn from_params_vector(params: &[f64]) -> Self {
        if params.len() != 1 {
            panic!("PrescribedCirculationShape::from_params_vector expects a vector of length 2");
        }

        PrescribedCirculationShape {
            inner_power: 2.0,
            outer_power: params[0],
        }
    }

    pub fn function_to_curve_fit(s: f64, params: &[f64]) -> f64 {
        let shape = PrescribedCirculationShape::from_params_vector(params);
        
        shape.value(s) / shape.mean_value()
    }

    pub fn from_curve_fit(x_data: &[f64], y_data: &[f64], initial_params: &[f64]) -> Self {        
        let y_data_mean = y_data.iter().sum::<f64>() / y_data.len() as f64;

        let y_data_normalized: Vec<f64> = y_data.iter().map(|&y| y / y_data_mean).collect();
        
        let curve_fitter = CurveFit{
            function: PrescribedCirculationShape::function_to_curve_fit,
            max_iterations: 20,
            delta_params: 0.00001,
            initial_damping_factor: 1000.0,
            damping_change_factor: 1.5,
            param_bounds: Some(vec![
                BoundedVariable { min: 0.1, max: 1.0 },
            ]),
            tolerance: 1e-12,
            max_step_size: 0.1,
        };

        let resulting_params = curve_fitter.fit_parameters(
            x_data, 
            &y_data_normalized, 
            initial_params
        );

        Self::from_params_vector(&resulting_params)
    }
}