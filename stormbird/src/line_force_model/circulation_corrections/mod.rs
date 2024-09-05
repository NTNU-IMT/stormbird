// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub mod prescribed_circulation;
pub mod smoothing;

use serde::{Serialize, Deserialize};

use prescribed_circulation::PrescribedCirculationShape;
use smoothing::SmoothingSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CirculationCorrection {
    None,
    PrescribedCirculation(PrescribedCirculationShape),
    Smoothing(SmoothingSettings),
}

impl CirculationCorrection {
    
}

impl Default for CirculationCorrection {
    fn default() -> Self {
        CirculationCorrection::None
    }
}

