// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};
use crate::vec3::Vec3;

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
    pub fn mirrored_point(&self, point: Vec3) -> Option<Vec3> {
        match self {
            SymmetryCondition::NoSymmetry => None,
            SymmetryCondition::X => Some(Vec3{x: -point.x, y:  point.y, z:  point.z}),
            SymmetryCondition::Y => Some(Vec3{x:  point.x, y: -point.y, z:  point.z}),
            SymmetryCondition::Z => Some(Vec3{x:  point.x, y:  point.y, z: -point.z}),
        }
    }

    pub fn corrected_velocity(&self, u_i: Vec3, u_i_m: Vec3) -> Vec3 {
        match self {
            SymmetryCondition::NoSymmetry => u_i,
            SymmetryCondition::X => Vec3{x: u_i.x - u_i_m.x, y: u_i.y + u_i_m.y, z: u_i.z + u_i_m.z},
            SymmetryCondition::Y => Vec3{x: u_i.x + u_i_m.x, y: u_i.y - u_i_m.y, z: u_i.z + u_i_m.z},
            SymmetryCondition::Z => Vec3{x: u_i.x + u_i_m.x, y: u_i.y + u_i_m.y, z: u_i.z - u_i_m.z},
        }
    }
}