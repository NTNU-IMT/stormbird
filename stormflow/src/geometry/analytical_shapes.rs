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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Box3D {
    pub center: SpatialVector,
    pub half_extents: SpatialVector, // (hx, hy, hz)
}

impl Box3D {
    pub fn signed_distance(&self, point: SpatialVector) -> Float {
        // Translate point into box-local space
        let p = point - self.center;

        // Per-axis distance to the box surface
        let q = SpatialVector::new(
            p[0].abs() - self.half_extents[0],
            p[1].abs() - self.half_extents[1],
            p[2].abs() - self.half_extents[2],
        );

        // Positive components of q = outside the box on that axis
        let outside = SpatialVector::new(
            q[0].max(0.0),
            q[1].max(0.0),
            q[2].max(0.0),
        );

        // Negative components of q = inside the box on that axis
        let inside_dist = q[0].max(q[1]).max(q[2]).min(0.0);

        outside.length() + inside_dist
    }
}