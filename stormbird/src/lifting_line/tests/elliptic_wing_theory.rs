// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::type_aliases::Float;
use stormath::consts::PI;

/// Structure used to calculate the lift and drag coefficients for an elliptic wing using the
/// elliptic wing theory.
pub struct EllipticWingTheory {
    pub cl_2d: Float,
    pub aspect_ratio: Float
}

impl EllipticWingTheory {
    pub fn cl(&self) -> Float {
        self.cl_2d / (1.0 + 2.0 / self.aspect_ratio)
    }

    pub fn cd(&self) -> Float {
        self.cl().powi(2) / (PI * self.aspect_ratio)
    }
}