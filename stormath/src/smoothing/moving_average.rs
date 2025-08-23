use std::collections::VecDeque;

use crate::type_aliases::Float;

#[derive(Debug, Default, Clone)]
/// A simple moving average filter
pub struct MovingAverage {
    window: VecDeque<Float>,
    window_size: usize,
}

impl MovingAverage {
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    pub fn add(&mut self, value: Float) {
        self.window.push_back(value);

        if self.window.len() > self.window_size {
            self.window.pop_front();
        }
    }

    pub fn get_average(&self) -> Float {
        self.window.iter().sum::<Float>() / self.window.len() as Float
    }
}