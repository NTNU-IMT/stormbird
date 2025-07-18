
use serde::{Serialize, Deserialize};

use stormath::smoothing::{
    gaussian::GaussianSmoothing,
    polynomial::CubicPolynomialSmoothing,
};

use super::prescribed::PrescribedCirculation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmoothingType {
    Gaussian(Vec<GaussianSmoothing<f64>>),
    CubicPolynomial(Vec<CubicPolynomialSmoothing<f64>>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CirculationSmoothing {
    pub smoothing_type: SmoothingType,
    pub prescribed_to_subtract_before_smoothing: Option<PrescribedCirculation>,
}