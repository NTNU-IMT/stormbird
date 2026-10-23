// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Moving average filter.

use std::collections::VecDeque;

use crate::type_aliases::Float;

#[derive(Debug, Default, Clone)]
/// A simple moving average filter
pub struct MovingAverage {
    window: VecDeque<Float>,
    window_size: usize,
}

impl MovingAverage {
    /// Creates a new filter that averages over the given number of values
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Adds a new value, and removes the oldest value if the window is full
    pub fn add(&mut self, value: Float) {
        self.window.push_back(value);

        if self.window.len() > self.window_size {
            self.window.pop_front();
        }
    }

    /// Returns the average of the values in the window
    pub fn get_average(&self) -> Float {
        self.window.iter().sum::<Float>() / self.window.len() as Float
    }
}