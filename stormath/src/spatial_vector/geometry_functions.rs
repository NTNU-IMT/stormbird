
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Geometry functions based on spatial vectors, such as areas.

use super::*;

#[inline(always)]
/// Calculates the area of a quadrilateral, based on the corner points given in order around the
/// edge. The area is half the length of the cross product of the two diagonals, which is exact for
/// all planar quadrilaterals, also when they are skewed. For a non-planar quadrilateral, it is the
/// area of the projection onto the plane normal to the cross product of the diagonals, which is the
/// same plane that [`normal_of_quadrilateral`] gives the normal of.
pub fn area_of_quadrilateral(vertices: &[SpatialVector; 4]) -> Float {
    let diagonal_1 = vertices[2] - vertices[0];
    let diagonal_2 = vertices[3] - vertices[1];

    0.5 * diagonal_1.cross(diagonal_2).length()
}

#[inline(always)]
/// Function that calculates the normal of a quadrilateral, based on the corner points
pub fn normal_of_quadrilateral(vertices: &[SpatialVector; 4]) -> SpatialVector {
    let a = vertices[2] - vertices[0];
    let b = vertices[3] - vertices[1];

    a.cross(b).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// The area of a rectangle and of a skewed parallelogram, where the previous implementation,
    /// based on Brahmagupta's formula, gave the product of the side lengths
    fn area_of_quadrilateral_is_exact_for_skewed_shapes() {
        let rectangle = [
            SpatialVector::new(0.0, 0.0, 0.0),
            SpatialVector::new(2.0, 0.0, 0.0),
            SpatialVector::new(2.0, 3.0, 0.0),
            SpatialVector::new(0.0, 3.0, 0.0),
        ];

        assert!((area_of_quadrilateral(&rectangle) - 6.0).abs() < 1e-6);

        // Unit sides, skewed 30 degrees from the x-axis: area = sin(30 deg) = 0.5
        let angle: Float = 30.0_f64.to_radians() as Float;
        let side = SpatialVector::new(angle.cos(), angle.sin(), 0.0);

        let parallelogram = [
            SpatialVector::new(0.0, 0.0, 0.0),
            SpatialVector::new(1.0, 0.0, 0.0),
            SpatialVector::new(1.0, 0.0, 0.0) + side,
            side,
        ];

        assert!((area_of_quadrilateral(&parallelogram) - 0.5).abs() < 1e-6);
    }
}
