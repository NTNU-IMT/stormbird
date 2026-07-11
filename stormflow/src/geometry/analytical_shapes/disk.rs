use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub center: SpatialVector,
    pub normal: SpatialVector, // Unit normal defining the plane of the disk
    pub radius: Float,
}

impl Disk {
    pub fn signed_distance(&self, point: SpatialVector) -> Float {
        // Vector from center to point
        let v = point - self.center;

        // Distance along the normal (height above/below the plane)
        let height = v.dot(self.normal);

        // Project point onto the plane of the disk
        let projected = v - self.normal * height;

        // Distance from center in the plane
        let radial_dist = projected.length();

        // Distance from the edge of the disk in the plane (0 if inside radius)
        let edge_dist = (radial_dist - self.radius).max(0.0);

        // Combined distance: Pythagorean theorem
        // When inside the radius, edge_dist is 0, so we get |height|
        // When outside, we get the distance to the nearest point on the disk edge
        (height * height + edge_dist * edge_dist).sqrt()
    }
}
