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
/// A 3D vector with typical geometric functions implemented
pub struct SpatialVector<const N: usize>(pub [f64; N]);


/// Convert from a 3-element array to a Vec3
impl<const N: usize> From<[f64; N]> for SpatialVector<N> {
    fn from(array: [f64; N]) -> Self {
        Self(array)
    }
}

/// Convert from a Vec3 to a 3-element array
impl<const N: usize> From<SpatialVector<N>> for [f64; N] {
    fn from(vector: SpatialVector<N>) -> [f64; N] {
        vector.0
    }
}


impl SpatialVector<2> {
    pub fn new(x: f64, y: f64) -> Self {
        Self([x, y])
    }

    pub fn unit_x() -> Self {
        Self([1.0, 0.0])
    }

    pub fn unit_y() -> Self {
        Self([0.0, 1.0])
    }
}

impl SpatialVector<3> {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self([x, y, z])
    }

    pub fn unit_x() -> Self {
        Self([1.0, 0.0, 0.0])
    }

    pub fn unit_y() -> Self {
        Self([0.0, 1.0, 0.0])
    }

    pub fn unit_z() -> Self {
        Self([0.0, 0.0, 1.0])
    }
}

impl<const N: usize> Default for SpatialVector<N> {
    fn default() -> Self {
        Self([0.0; N])
    }
}

impl<const N: usize> std::fmt::Display for SpatialVector<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}




