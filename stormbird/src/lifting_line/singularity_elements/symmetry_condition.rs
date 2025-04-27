// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};
use stormath::spatial_vector::SpatialVector;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Enum that represent a symmetry plane. 
/// 
/// To be used when calculating induced velocities in space from a potential theory velocity model.
pub enum SymmetryCondition {
    NoSymmetry,
    X,
    Y,
    Z,
}

impl Default for SymmetryCondition {
    fn default() -> Self {
        SymmetryCondition::NoSymmetry
    }
}

impl SymmetryCondition {
    pub fn mirrored_point(&self, point: SpatialVector<3>) -> Option<SpatialVector<3>> {
        match self {
            SymmetryCondition::NoSymmetry => None,
            SymmetryCondition::X => Some(SpatialVector::<3>::new(-point[0], point[1], point[2])),
            SymmetryCondition::Y => Some(SpatialVector::<3>::new(point[0], -point[1], point[2])),
            SymmetryCondition::Z => Some(SpatialVector::<3>::new(point[0], point[1], -point[2])),
        }
    }

    pub fn corrected_velocity(&self, u_i: SpatialVector<3>, u_i_m: SpatialVector<3>) -> SpatialVector<3> {
        match self {
            SymmetryCondition::NoSymmetry => u_i,
            SymmetryCondition::X => SpatialVector::<3>::new(u_i[0] - u_i_m[0], u_i[1] + u_i_m[1], u_i[2] + u_i_m[2]),
            SymmetryCondition::Y => SpatialVector::<3>::new(u_i[0] + u_i_m[0], u_i[1] - u_i_m[1], u_i[2] + u_i_m[2]),
            SymmetryCondition::Z => SpatialVector::<3>::new(u_i[0] + u_i_m[0], u_i[1] + u_i_m[1], u_i[2] - u_i_m[2]),
        }
    }
}