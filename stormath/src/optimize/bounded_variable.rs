
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A structure used to represent a bounded variable to be used within optimization algorithms.
pub struct BoundedVariable {
    pub min: f64,
    pub max: f64,
}

impl BoundedVariable {
     /// Transform unbounded parameter to bounded using logistic function
    pub fn transform_to_bounded(&self, unbounded: f64) -> f64 {
        if self.min.is_infinite() && self.max.is_infinite() {
            unbounded
        } else if self.min.is_infinite() {
            self.max - (-unbounded).exp()
        } else if self.max.is_infinite() {
            self.min + unbounded.exp()
        } else {
            self.min + (self.max - self.min) / (1.0 + (-unbounded).exp())
        }
    }

    /// Transform bounded parameter back to unbounded space
    pub fn transform_to_unbounded(&self, bounded: f64) -> f64 {
        if self.min.is_infinite() && self.max.is_infinite() {
            bounded
        } else if self.min.is_infinite() {
            -(self.max - bounded).ln()
        } else if self.max.is_infinite() {
            (bounded - self.min).ln()
        } else {
            let ratio = (bounded - self.min) / (self.max - self.min);
            (ratio / (1.0 - ratio)).ln()
        }
    }
}
