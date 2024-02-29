// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::iter;
use crate::vec3::Vec3;

impl iter::Sum for Vec3 {
    fn sum<I>(iter: I) -> Self 
    where 
        I: Iterator<Item = Self> {
        iter.fold(Self { x: 0.0, y: 0.0, z:0.0 }, |a, b| Self {
            x: a.x + b.x,
            y: a.y + b.y,
            z: a.z + b.z,
        })
    }
}


impl<'a> iter::Sum<&'a Self> for Vec3 {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(Self { x: 0.0, y: 0.0, z: 0.0 }, |a, b| Self {
            x: a.x + b.x,
            y: a.y + b.y,
            z: a.z + b.z,
        })
    }
}