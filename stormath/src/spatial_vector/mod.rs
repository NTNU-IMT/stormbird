// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality related to vectors representing spatial quantities.
//! 
//! The data structure does not assume anything about the number of dimensions, meaning it can be 
//! used both for 2D and 3D vectors.

pub mod operators;
pub mod serde_implementations;
pub mod vector_math;
pub mod transformations;
pub mod geometry_functions;
pub mod iterators;

#[derive(Copy, Clone, Debug, PartialEq)]
/// A 2D or 3D vector with typical geometric functions implemented
pub struct SpatialVector(pub [f64; 4]);


/// Convert from a 3-element array to a Vec3
impl From<[f64; 3]> for SpatialVector {
    fn from(array: [f64; 3]) -> Self {
        let mut data = [0.0; 4];
        
        for i in 0..3 {
            data[i] = array[i];
        }

        Self(data)
    }
}

/// Convert from a Vec3 to a 3-element array
impl From<SpatialVector> for [f64; 3] {
    fn from(vector: SpatialVector) -> [f64; 3] {
        let mut out = [0.0; 3];

        for i in 0..3 {
            out[i] = vector.0[i]
        }

        out
    }
}


impl SpatialVector {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self([x, y, z, 0.0])
    }

    pub fn unit_x() -> Self {
        Self([1.0, 0.0, 0.0, 0.0])
    }

    pub fn unit_y() -> Self {
        Self([0.0, 1.0, 0.0, 0.0])
    }

    pub fn unit_z() -> Self {
        Self([0.0, 0.0, 1.0, 0.0])
    }
}

impl Default for SpatialVector {
    fn default() -> Self {
        Self([0.0; 4])
    }
}

impl std::fmt::Display for SpatialVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}




