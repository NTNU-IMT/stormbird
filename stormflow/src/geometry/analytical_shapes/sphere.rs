use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

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