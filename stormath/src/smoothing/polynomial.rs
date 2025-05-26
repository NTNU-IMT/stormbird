use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WindowSize {
    Five,
    Seven,
    Nine
}

impl WindowSize {
    pub fn from_str(window_size: &str) -> Self {
        match window_size {
            "Five" => Self::Five,
            "Seven" => Self::Seven,
            "Nine" => Self::Nine,
            _ => panic!("Unknown window size: {}", window_size)
        }
    }

    pub fn weights(&self) -> Vec<f64> {
        match self {
            Self::Five => vec![-3.0, 12.0, 17.0, 12.0, -3.0],
            Self::Seven => vec![-2.0, 3.0, 6.0, 7.0, 6.0, 3.0, -2.0],
            Self::Nine => vec![-21.0, 14.0, 39.0, 54.0, 59.0, 54.0, 39.0, 14.0, -21.0]
        }
    }

    pub fn normalization(&self) -> f64 {
        match self {
            Self::Five => 35.0,
            Self::Seven => 21.0,
            Self::Nine => 231.0
        }
    }

    pub fn window_offset(&self) -> usize {
        match self {
            Self::Five => 2,
            Self::Seven => 3,
            Self::Nine => 4
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CubicPolynomialSmoothing {
    pub window_size: WindowSize,
    pub end_conditions: [EndCondition; 2]
}

impl CubicPolynomialSmoothing {
    pub fn apply_smoothing<T>(&self, y: &[T]) -> Vec<T>
    where T: SmoothingOps
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