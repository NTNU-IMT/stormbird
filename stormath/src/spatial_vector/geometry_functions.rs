
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

#[inline(always)]
/// Function that calculates the induced velocity from a vortex panel, based on the corner points
/// given as input. Source: <https://en.wikipedia.org/wiki/Brahmagupta%27s_formula>
pub fn area_of_quadrilateral(vertices: &[SpatialVector; 4]) -> Float {
    let a = (vertices[1] - vertices[0]).length();
    let b = (vertices[2] - vertices[1]).length();
    let c = (vertices[3] - vertices[2]).length();
    let d = (vertices[0] - vertices[3]).length();

    // Calculate the semi-perimeter
    let s = (a + b + c + d) * 0.5;

    ((s - a) * (s - b) * (s - c) * (s - d)).sqrt()
}

#[inline(always)]
/// Function that calculates the normal of a quadrilateral, based on the corner points
pub fn normal_of_quadrilateral(vertices: &[SpatialVector; 4]) -> SpatialVector {
    let a = vertices[2] - vertices[0];
    let b = vertices[3] - vertices[1];

    a.cross(b).normalize()
}