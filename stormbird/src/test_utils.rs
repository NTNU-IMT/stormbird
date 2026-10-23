// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Utilities shared between the tests in the library.

use stormath::type_aliases::Float;

/// Returns the tolerance to use in a test, depending on whether the library is compiled with
/// double or single precision. Single precision needs larger tolerances due to round-off errors.
pub fn tolerance(double_precision: Float, single_precision: Float) -> Float {
    if cfg!(feature = "single_precision") {
        single_precision
    } else {
        double_precision
    }
}
