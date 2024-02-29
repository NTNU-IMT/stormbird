// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

pub mod power_loss;

use power_loss::PowerLoss;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum TipLossFactor {
    #[default]
    None,
    PowerLoss(PowerLoss),
}

impl TipLossFactor {
    /// Returns a corrected value for the circulation distribution based on the chosen tip loss 
    /// model and the non-dimensional span wise length.
    pub fn correct_circulation_distribution(&self, non_dim_span: &[f64], circulation_strength: &[f64]) -> Vec<f64> {
        match self {
            Self::None => circulation_strength.to_vec(),
            Self::PowerLoss(power_loss) => {
                non_dim_span.iter().zip(circulation_strength.iter()).map(|(span, strength)| {
                    strength * power_loss.correction_value(*span)
                }).collect()
            },
        }
    }
}

