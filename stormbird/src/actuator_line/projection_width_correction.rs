// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elliptic {
    min_value: f64,
    both_ends: bool
}

impl Elliptic {
    pub fn width_correction(&self, non_dim_span: f64) -> f64 {
        let x = if self.both_ends {
            non_dim_span.abs()
        } else {
            ((non_dim_span + 0.5) / 2.0).abs()
        };

        let s = (1.0 - (2.0 * x).powi(2)).sqrt();

        1.0 * s + (1.0 - s) * self.min_value
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ProjectionWidthCorrection {
    #[default]
    None,
    Elliptic(Elliptic),
}

impl ProjectionWidthCorrection {
    pub fn width_correction(&self, non_dim_span: &[f64]) -> Vec<f64> {
        match self {
            ProjectionWidthCorrection::None => vec![1.0; non_dim_span.len()],
            ProjectionWidthCorrection::Elliptic(elliptic) => {
                non_dim_span.iter().map(|&x| elliptic.width_correction(x)).collect()
            },
        }
    }
}