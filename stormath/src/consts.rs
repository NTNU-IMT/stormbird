// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Different constants. Defined so that it should be easy to load these with the Float type alias

use super::type_aliases::Float;

#[cfg(feature = "single_precision")]
/// Archimedes' constant (π)
pub const PI: Float = std::f32::consts::PI;
#[cfg(feature = "single_precision")]
/// The full circle constant (τ = 2π)
pub const TAU: Float = std::f32::consts::TAU;
#[cfg(feature = "single_precision")]
/// Positive infinity
pub const INFINITY: Float = f32::INFINITY;
#[cfg(feature = "single_precision")]
/// Smallest positive normal value
pub const MIN_POSITIVE: Float = f32::MIN_POSITIVE;
#[cfg(feature = "single_precision")]
/// Smallest finite value
pub const MIN: Float = f32::MIN;

#[cfg(not(feature = "single_precision"))]
/// Archimedes' constant (π)
pub const PI: Float = std::f64::consts::PI;
#[cfg(not(feature = "single_precision"))]
/// The full circle constant (τ = 2π)
pub const TAU: Float = std::f64::consts::TAU;
#[cfg(not(feature = "single_precision"))]
/// Positive infinity
pub const INFINITY: Float = f64::INFINITY;
#[cfg(not(feature = "single_precision"))]
/// Smallest positive normal value
pub const MIN_POSITIVE: Float = f64::MIN_POSITIVE;
#[cfg(not(feature = "single_precision"))]
/// Smallest finite value
pub const MIN: Float = f64::MIN;
