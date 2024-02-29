// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

pub struct EllipticWingTheory {
    pub cl_2d: f64,
    pub aspect_ratio: f64
}

impl EllipticWingTheory {
    pub fn cl(&self) -> f64 {
        self.cl_2d / (1.0 + 2.0 / self.aspect_ratio)
    }

    pub fn cd(&self) -> f64 {
        self.cl().powi(2) / (PI * self.aspect_ratio)
    }
}