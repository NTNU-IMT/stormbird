// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality related to vectors representing spatial quantities, e.g., position, velocity and
//! acceleration.
//! 
//! The data structure does not assume anything about the number of dimensions, meaning it can be 
//! used both for 2D and 3D vectors.

pub mod operators;
pub mod serde_implementations;
pub mod vector_math;
pub mod transformations;
pub mod geometry_functions;
pub mod iterators;

/// Number of components in the vector
pub const VECTOR_LENGTH: usize = 3;

#[cfg(feature = "padded_spatial_vectors")]
/// Size of the data array. Padded to four values, as the `padded_spatial_vectors` feature is
/// enabled.
pub const DATA_SIZE: usize = 4;

#[cfg(not(feature = "padded_spatial_vectors"))]
/// Size of the data array. Equal to the vector length by default, or padded to four values with
/// the `padded_spatial_vectors` feature.
pub const DATA_SIZE: usize = 3;

use crate::type_aliases::Float;

#[derive(Copy, Clone, Debug, PartialEq)]
/// A 2D or 3D vector with typical geometric functions implemented
pub struct SpatialVector(pub [Float; DATA_SIZE]);

/// Convert from a 3-element array to a SpatialVector
impl From<[Float; VECTOR_LENGTH]> for SpatialVector {
    fn from(array: [Float; VECTOR_LENGTH]) -> Self {
        let mut data = [0.0; DATA_SIZE];

        data[..VECTOR_LENGTH].copy_from_slice(&array);

        Self(data)
    }
}

/// Convert from a SpatialVector to a 3-element array
impl From<SpatialVector> for [Float; VECTOR_LENGTH] {
    fn from(vector: SpatialVector) -> [Float; VECTOR_LENGTH] {
        let mut out = [0.0; VECTOR_LENGTH];

        out.copy_from_slice(&vector.0[..VECTOR_LENGTH]);

        out
    }
}

impl SpatialVector {
    /// Creates a new vector from its components
    pub fn new(x: Float, y: Float, z: Float) -> Self {
        let mut data = [0.0; DATA_SIZE];
        data[0] = x;
        data[1] = y;
        data[2] = z;

        Self(data)
    }

    /// Returns the unit vector along the x-axis
    pub fn unit_x() -> Self {
        let mut data = [0.0; DATA_SIZE];

        data[0] = 1.0;

        Self(data)
    }

    /// Returns the unit vector along the y-axis
    pub fn unit_y() -> Self {
        let mut data = [0.0; DATA_SIZE];

        data[1] = 1.0;

        Self(data)
    }

    /// Returns the unit vector along the z-axis
    pub fn unit_z() -> Self {
        let mut data = [0.0; DATA_SIZE];

        data[2] = 1.0;

        Self(data)
    }

    /// Sets all NaN values in the vector to zero
    pub fn replace_nans_with_zeros(&mut self) {
        for i in 0..DATA_SIZE {
            if self.0[i].is_nan() {
                self.0[i] = 0.0;
            }
        }
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
