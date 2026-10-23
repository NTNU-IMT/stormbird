// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Smoothing by fitting cubic polynomials to a moving window of points, using the
//! Savitzky-Golay filter.
//!
//! Source: <https://en.wikipedia.org/wiki/Savitzky%E2%80%93Golay_filter>

use super::*;

use std::str::FromStr;

use crate::error::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Number of points in the smoothing window
pub enum WindowSize {
    /// Five points
    Five,
    /// Seven points
    Seven,
    /// Nine points
    Nine
}

impl FromStr for WindowSize {
    type Err = Error;

    /// Creates a window size from its name
    fn from_str(window_size: &str) -> Result<Self, Self::Err> {
        match window_size {
            "Five" => Ok(Self::Five),
            "Seven" => Ok(Self::Seven),
            "Nine" => Ok(Self::Nine),
            _ => Err(Error::InvalidInput(format!("unknown window size: {}", window_size)))
        }
    }
}

impl WindowSize {
    /// Returns the (non-normalized) weights of the points in the window
    pub fn weights(&self) -> Vec<Float> {
        match self {
            Self::Five => vec![-3.0, 12.0, 17.0, 12.0, -3.0],
            Self::Seven => vec![-2.0, 3.0, 6.0, 7.0, 6.0, 3.0, -2.0],
            Self::Nine => vec![-21.0, 14.0, 39.0, 54.0, 59.0, 54.0, 39.0, 14.0, -21.0]
        }
    }

    /// Returns the normalization factor for the weights
    pub fn normalization(&self) -> Float {
        match self {
            Self::Five => 35.0,
            Self::Seven => 21.0,
            Self::Nine => 231.0
        }
    }

    /// Returns the number of points on each side of the center point in the window
    pub fn window_offset(&self) -> usize {
        match self {
            Self::Five => 2,
            Self::Seven => 3,
            Self::Nine => 4
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Settings for cubic polynomial smoothing
pub struct CubicPolynomialSmoothing<T: SmoothingOps> {
    /// Number of points in the smoothing window
    pub window_size: WindowSize,
    /// How to treat the start and the end of the data
    pub end_conditions: [EndCondition<T>; 2]
}

impl<T: SmoothingOps> CubicPolynomialSmoothing<T> {
    /// Returns the smoothed data. The data is assumed to be equally spaced.
    pub fn apply_smoothing(&self, y: &[T]) -> Vec<T>
    {
        let n = y.len();

        let window_offset = self.window_size.window_offset();
        let number_of_end_insertions = window_offset;

        let y_modified = EndCondition::add_end_values_to_y_data(
            y, number_of_end_insertions, self.end_conditions
        );

        let weights = self.window_size.weights();
        let normalization = self.window_size.normalization();

        let mut y_smooth: Vec<T> = Vec::with_capacity(n);

        for i in 0..n {
            let mut y_smooth_i: T = Default::default();

            let i_mod = i + number_of_end_insertions;

            for j in 0..weights.len() {
                y_smooth_i = y_smooth_i + y_modified[i_mod+j-window_offset] * weights[j];
            }

            y_smooth.push(y_smooth_i / normalization);
        }

        y_smooth
    }
}