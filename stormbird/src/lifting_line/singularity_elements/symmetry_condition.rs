// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Enum that represent a symmetry plane.
///
/// To be used when calculating induced velocities in space from a potential theory velocity model.
#[derive(Default)]
pub enum SymmetryCondition {
    /// No symmetry
    #[default]
    NoSymmetry,
    /// Symmetry plane at x = 0
    X,
    /// Symmetry plane at y = 0
    Y,
    /// Symmetry plane at z = 0
    Z,
}


impl SymmetryCondition {
    /// Returns the point mirrored about the symmetry plane, or `None` if there is no symmetry
    pub fn mirrored_point(&self, point: SpatialVector) -> Option<SpatialVector> {
        match self {
            SymmetryCondition::NoSymmetry => None,
            SymmetryCondition::X => Some(SpatialVector::new(-point[0], point[1], point[2])),
            SymmetryCondition::Y => Some(SpatialVector::new(point[0], -point[1], point[2])),
            SymmetryCondition::Z => Some(SpatialVector::new(point[0], point[1], -point[2])),
        }
    }

    /// Combines the velocity induced at a point, `u_i`, with the velocity induced at the mirrored
    /// point, `u_i_m`, to get the velocity with the symmetry condition
    pub fn corrected_velocity(&self, u_i: SpatialVector, u_i_m: SpatialVector) -> SpatialVector {
        match self {
            SymmetryCondition::NoSymmetry => u_i,
            SymmetryCondition::X => SpatialVector::new(u_i[0] - u_i_m[0], u_i[1] + u_i_m[1], u_i[2] + u_i_m[2]),
            SymmetryCondition::Y => SpatialVector::new(u_i[0] + u_i_m[0], u_i[1] - u_i_m[1], u_i[2] + u_i_m[2]),
            SymmetryCondition::Z => SpatialVector::new(u_i[0] + u_i_m[0], u_i[1] + u_i_m[1], u_i[2] - u_i_m[2]),
        }
    }
}
