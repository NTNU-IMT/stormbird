use serde::{Serialize, Deserialize};

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

    /// Returns the circulation distribution based on the relative span distance
    pub fn get_values(&self, relative_span_distance: &[f64]) -> Vec<f64> {
        let mut values = vec![0.0; relative_span_distance.len()];

        for (i, x) in relative_span_distance.iter().enumerate() {
            if x.abs() < 0.5 {
                values[i] = (1.0 - (2.0 * x.abs()).powf(self.inner_power)).powf(self.outer_power);
            }
        }

        values
    }
}