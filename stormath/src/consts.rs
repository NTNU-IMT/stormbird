// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Different constant. Defined so that it should be easy load these with the Float type alias

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
