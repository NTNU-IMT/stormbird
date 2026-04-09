
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

use serde::{Serialize, Deserialize};

/// Blending function that goes between 0 and 1 from -epsilon to epsilon
pub fn blending_function(distance: Float, epsilon: Float) -> Float {
    if distance > epsilon {
        1.0
    } else if distance < -epsilon {
        0.0
    } else {
        0.5 * (1.0 + distance/epsilon + (PI * distance / epsilon).sin() / PI)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sphere {
    pub center: SpatialVector,
    pub radius: Float,
}

impl Sphere {
    pub fn signed_distance(&self, point: SpatialVector) -> Float {
        let distance_from_center = (point - self.center).length();
        
        distance_from_center - self.radius
    }
}
