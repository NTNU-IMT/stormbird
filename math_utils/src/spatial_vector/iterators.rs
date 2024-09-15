// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::iter;
use super::*;

impl<const N: usize> iter::Sum for SpatialVector<N> {
    fn sum<I>(iter: I) -> Self 
    where 
        I: Iterator<Item = Self> 
    {
        iter.fold(
            Self([0.0; N]), |a, b| {
                let mut result = [0.0; N];

                for i in 0..N {
                    result[i] = a[i] + b[i];
                }

                Self(result)
            }
        )
    }
}

impl<'a, const N: usize> iter::Sum<&'a Self> for SpatialVector<N> {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(
            Self([0.0; N]), |a, b| {
                let mut result = [0.0; N];

                for i in 0..N {
                    result[i] = a[i] + b[i];
                }

                Self(result)
            }
        )
    }
}