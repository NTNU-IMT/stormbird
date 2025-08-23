
// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of some of the theoretical functions from elliptic wing theory, based on the 
//! classical lifting line theory.

use stormath::{
    type_aliases::Float,
    consts::PI
};

pub struct EllipticalWing {
    pub aspect_ratio: Float,
}

impl EllipticalWing {
    #[inline(always)]
    /// Function that computes the lift-induced angle of attack according to elliptic wing theory, based
    /// only on the lift coefficient and aspect ratio of the wing.
    pub fn lift_induced_angle_of_attach(&self, lift_coefficient: Float) -> Float {
        if self.aspect_ratio <= 0.0 {
            return 0.0;
        }

        lift_coefficient / (PI * self.aspect_ratio)
    }
}