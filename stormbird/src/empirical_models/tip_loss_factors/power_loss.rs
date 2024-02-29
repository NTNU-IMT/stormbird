// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerLoss {
    pub outer_power: f64,
    pub inner_power: f64,
    pub max_value: f64,
    pub min_value: f64,
    pub correct_both_ends: bool,
}

impl PowerLoss {
    pub fn correction_value(&self, non_dim_span: f64) -> f64 {
        let x = if self.correct_both_ends {
            non_dim_span.abs()
        } else {
            ((non_dim_span + 0.5) / 2.0).abs()
        };

        let s = (1.0 - (2.0 * x).powf(self.inner_power)).powf(self.outer_power);

        self.max_value * s + (1.0 - s) * self.min_value
    }
}

impl Default for PowerLoss {
    fn default() -> Self {
        Self {
            outer_power: 0.5,
            inner_power: 2.0,
            max_value: 1.0,
            min_value: 0.0,
            correct_both_ends: true,
        }
    }
}