
use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

use crate::line_force_model::span_line::SpanLine;

use serde::{Serialize, Deserialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpanflowDamper {
    #[serde(default)]
    pub linear_coefficient: Float,
    #[serde(default)]
    pub squared_coefficient: Float,
}

impl SpanflowDamper {
    pub fn get_sectional_force(
        &self, 
        chord_length: Float,
        span_line: SpanLine,
        velocity: SpatialVector,
        density: Float
    ) -> SpatialVector {
        let common = 0.5 * chord_length * span_line.length() * density;

        let spanwise_flow = velocity.project(span_line.relative_vector());

        let coefficients = self.linear_coefficient + self.squared_coefficient * spanwise_flow.length();

        let force = common * coefficients * spanwise_flow;

        force
    }
}