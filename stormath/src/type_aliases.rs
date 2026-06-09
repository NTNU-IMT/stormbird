// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


//! Type aliases to easy switch between different floating point number representations

#[cfg(feature = "single_precision")]
pub type Float = f32;

#[cfg(not(feature = "single_precision"))]
pub type Float = f64;
