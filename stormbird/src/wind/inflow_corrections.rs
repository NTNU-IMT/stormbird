use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflowCorrectionSingleSail {
    pub non_dimensional_span_distances: Vec<f64>,
    pub magnitude_corrections: Vec<f64>,
    pub angle_corrections: Vec<f64>,
}