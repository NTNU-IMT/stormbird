// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! Type aliases to easily switch between different floating point number representations

#[cfg(feature = "single_precision")]
/// Floating point type used in the library. Single precision, as the `single_precision` feature is enabled.
pub type Float = f32;

#[cfg(not(feature = "single_precision"))]
/// Floating point type used in the library. Double precision by default, or single precision with the `single_precision` feature.
pub type Float = f64;
