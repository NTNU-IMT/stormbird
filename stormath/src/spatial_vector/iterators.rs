// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::iter;
use super::*;

impl iter::Sum for SpatialVector {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(
            Self([0.0; DATA_SIZE]), |a, b| {
                let mut result = [0.0; DATA_SIZE];

                for i in 0..VECTOR_LENGTH {
                    result[i] = a[i] + b[i];
                }

                Self(result)
            }
        )
    }
}

impl<'a,> iter::Sum<&'a Self> for SpatialVector {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(
            Self([0.0; DATA_SIZE]), |a, b| {
                let mut result = [0.0; DATA_SIZE];

                for i in 0..VECTOR_LENGTH {
                    result[i] = a[i] + b[i];
                }

                Self(result)
            }
        )
    }
}