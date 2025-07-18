// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AngleOfAttackCorrection {
    None,
    GaussianSmoothing
}

impl Default for AngleOfAttackCorrection {
    fn default() -> Self {
        AngleOfAttackCorrection::None
    }
}