

pub struct MovingAverage {
    window: VecDeque<f64>,
    window_size: usize,
}

impl MovingAverage {
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    pub fn add(&mut self, value: f64) {
        self.window.push_back(value);

        if self.window.len() > self.window_size {
            self.window.pop_front();
        }
    }

    pub fn get_average(&self) -> f64 {
        self.window.iter().sum::<f64>() / self.window.len() as f64
    }
}
