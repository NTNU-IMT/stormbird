use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disk {
    pub center: SpatialVector,
    pub normal: SpatialVector, // Unit normal defining the plane of the disk
    pub radius: Float,
    // Thickness of the disk, extruded from `center` along `normal` (i.e. spanning [0, thickness]).
    // Zero thickness recovers the flat disk.
    #[serde(default)]
    pub thickness: Float,
    // Fillet (rounding) radius applied to the edges created by the extrusion.
    #[serde(default)]
    pub fillet_radius: Float,
}

impl Disk {
    pub fn signed_distance(&self, point: SpatialVector) -> Float {
        // Vector from center to point
        let v = point - self.center;

        // Distance along the normal (height above/below the plane at `center`)
        let raw_height = v.dot(self.normal);

        // Project point onto the plane of the disk
        let projected = v - self.normal * raw_height;

        // Distance from center in the plane
        let radial_dist = projected.length();

        // The extrusion spans [0, thickness] along the normal, so the mid-plane
        // sits at half the thickness away from `center`.
        let half_thickness = self.thickness * 0.5;
        let height = raw_height - half_thickness;

        // Flat radius of the disk before rounding is applied, so that the rounded
        // shape's outer extent still reaches `radius`.
        let flat_radius = (self.radius - self.fillet_radius).max(0.0);

        // 2D signed-distance-to-rounded-box formula (radial vs. height axes), rounded
        // by `fillet_radius`. With fillet_radius = 0 and thickness = 0 this reduces
        // exactly to the original flat disk formula.
        let dx = radial_dist - flat_radius;
        let dy = height.abs() - half_thickness;

        let inside_dist = dx.max(dy).min(0.0);
        let outside_dist = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();

        inside_dist + outside_dist - self.fillet_radius
    }
}
