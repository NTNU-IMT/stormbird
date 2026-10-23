
use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

use crate::line_force_model::span_line::SpanLine;

use serde::{Serialize, Deserialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Model for a force that damps the flow along the span of the line elements
pub struct SpanflowDamper {
    /// Coefficient for the force that is linear in the spanwise velocity
    #[serde(default)]
    pub linear_coefficient: Float,
    /// Coefficient for the force that is quadratic in the spanwise velocity
    #[serde(default)]
    pub squared_coefficient: Float,
}

impl SpanflowDamper {
    /// Returns the damping force on a line element, based on the spanwise component of the velocity
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

        

        common * coefficients * spanwise_flow
    }
}