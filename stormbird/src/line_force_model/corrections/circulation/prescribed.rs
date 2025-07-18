use serde::{Serialize, Deserialize};

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

    /// Returns the circulation distribution based on the relative span distance
    pub fn get_values(&self, s: &[f64]) -> Vec<f64> {
        s.iter().map(|&x| self.value(x)).collect()
    }

    pub fn from_params_vector(params: &[f64]) -> Self {
        if params.len() != 2 {
            panic!("PrescribedCirculationShape::from_params_vector expects a vector of length 2");
        }

        PrescribedCirculationShape {
            inner_power: params[0],
            outer_power: params[1],
        }
    }
}