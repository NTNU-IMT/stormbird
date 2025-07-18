// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub mod prescribed;
pub mod smoothing;

use serde::{Serialize, Deserialize};

use prescribed::PrescribedCirculation;
use smoothing::{
    CirculationSmoothing,
    CirculationSmoothingBuilder,
};

use crate::line_force_model::LineForceModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum CirculationCorrectionBuilder {
    /// No correction applied to the circulation strength.
    None,
    /// The circulation distribution will be set to a prescribed shape, which will be scaled based
    /// on the estimated circulation.
    Prescribed(PrescribedCirculation),
    /// The raw estimated circulation distribution will be smoothed using a Gaussian kernel.
    Smoothing(CirculationSmoothingBuilder),
}

impl Default for CirculationCorrectionBuilder {
    fn default() -> Self {
        CirculationCorrectionBuilder::None
    }
}

impl CirculationCorrectionBuilder {
    pub fn build(&self, line_force_model: &LineForceModel) -> CirculationCorrection {
        match self {
            CirculationCorrectionBuilder::None => CirculationCorrection::None,
            CirculationCorrectionBuilder::Prescribed(prescribed) => {
                CirculationCorrection::Prescribed(prescribed.clone())
            }
            CirculationCorrectionBuilder::Smoothing(settings_builder) => {
                let settings = settings_builder.build(line_force_model);

                CirculationCorrection::Smoothing(settings)
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Enum for controlling what type of correction to apply to the estimated circulation distribution.
pub enum CirculationCorrection {
    /// Default, which is no correction.
    None,
    /// The circulation distribution will be set to a prescribed shape, which will be scaled based
    /// on the estimated circulation.
    Prescribed(PrescribedCirculation),
    /// The raw estimated circulation distribution will be smoothed using a Gaussian kernel.
    Smoothing(CirculationSmoothing)
}

impl Default for CirculationCorrection {
    fn default() -> Self {
        CirculationCorrection::None
    }
}

