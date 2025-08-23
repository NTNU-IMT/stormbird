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

// Length of the vector
pub const VECTOR_LENGTH: usize = 3;

#[cfg(feature = "padded_spatial_vectors")]
pub const DATA_SIZE: usize = 4;

#[cfg(not(feature = "padded_spatial_vectors"))]
pub const DATA_SIZE: usize = 3;

use crate::type_aliases::Float;

#[derive(Copy, Clone, Debug, PartialEq)]
/// A 2D or 3D vector with typical geometric functions implemented
pub struct SpatialVector(pub [Float; DATA_SIZE]);

/// Convert from a 3-element array to a Vec3
impl From<[Float; VECTOR_LENGTH]> for SpatialVector {
    fn from(array: [Float; VECTOR_LENGTH]) -> Self {
        let mut data = [0.0; DATA_SIZE];

        for i in 0..VECTOR_LENGTH {
            data[i] = array[i];
        }

        Self(data)
    }
}

/// Convert from a Vec3 to a 3-element array
impl From<SpatialVector> for [Float; VECTOR_LENGTH] {
    fn from(vector: SpatialVector) -> [Float; VECTOR_LENGTH] {
        let mut out = [0.0; VECTOR_LENGTH];

        for i in 0..VECTOR_LENGTH {
            out[i] = vector.0[i]
        }

        out
    }
}

impl SpatialVector {
    pub fn new(x: Float, y: Float, z: Float) -> Self {
        let mut data = [0.0; DATA_SIZE];
        data[0] = x;
        data[1] = y;
        data[2] = z;

        Self(data)
    }

    pub fn unit_x() -> Self {
        let mut data = [0.0; DATA_SIZE];

        data[0] = 1.0;

        Self(data)
    }

    pub fn unit_y() -> Self {
        let mut data = [0.0; DATA_SIZE];

        data[1] = 1.0;

        Self(data)
    }

    pub fn unit_z() -> Self {
        let mut data = [0.0; DATA_SIZE];

        data[2] = 1.0;

        Self(data)
    }
}

impl Default for SpatialVector {
    fn default() -> Self {
        Self([0.0; DATA_SIZE])
    }
}

impl std::fmt::Display for SpatialVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}




