
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Collection of common mathematical utility functions.
//!
//! The library is developed in parallel to, and for, the *Stormbird library*. The functions 
//! available is closely connected to what is needed here. However, the implementation of the 
//! functionality in this crate is such that it may also be useful in other contexts. It is 
//! therefore kept as a  separate crate. 

pub mod interpolation;
pub mod integration;
pub mod smoothing;
pub mod statistics;
pub mod array_generation;
pub mod finite_difference;
pub mod spatial_vector;
pub mod special_functions;
pub mod solvers;
pub mod matrix;
pub mod optimize;
pub mod rigid_body_motion;
pub mod error;

pub mod type_aliases {
    #[cfg(feature = "single_precision")]
    pub type Float = f32;

    #[cfg(not(feature = "single_precision"))]
    pub type Float = f64;    
}

pub mod consts {
    use super::type_aliases::Float;

    #[cfg(feature = "single_precision")]
    pub const PI: Float = std::f32::consts::PI;
    #[cfg(feature = "single_precision")]
    pub const TAU: Float = std::f32::consts::TAU;
    #[cfg(feature = "single_precision")]
    pub const INFINITY: Float = std::f32::INFINITY;
    #[cfg(feature = "single_precision")]
    pub const MIN_POSITIVE: Float = std::f32::MIN_POSITIVE;
    #[cfg(feature = "single_precision")]
    pub const MIN: Float = std::f32::MIN;

    #[cfg(not(feature = "single_precision"))]
    pub const PI: Float = std::f64::consts::PI;
    #[cfg(not(feature = "single_precision"))]
    pub const TAU: Float = std::f64::consts::TAU;
    #[cfg(not(feature = "single_precision"))]
    pub const INFINITY: Float = std::f64::INFINITY;
    #[cfg(not(feature = "single_precision"))]
    pub const MIN_POSITIVE: Float = std::f64::MIN_POSITIVE;
    #[cfg(not(feature = "single_precision"))]
    pub const MIN: Float = std::f64::MIN;

}
