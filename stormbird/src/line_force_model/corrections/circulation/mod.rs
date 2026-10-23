// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Prescribed shapes for the circulation distribution
pub mod prescribed;
/// Smoothing of the circulation distribution
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
#[derive(Default)]
/// Builder for the circulation correction. The smoothing settings depend on the geometry of the
/// wings, and are therefore set when the line force model is built.
pub enum CirculationCorrectionBuilder {
    /// No correction applied to the circulation strength.
    #[default]
    None,
    /// The circulation distribution will be set to a prescribed shape, which will be scaled based
    /// on the estimated circulation.
    Prescribed(PrescribedCirculation),
    /// The raw estimated circulation distribution will be smoothed, using either Gaussian or cubic
    /// polynomial smoothing.
    Smoothing(CirculationSmoothingBuilder),
}


impl CirculationCorrectionBuilder {
    /// Builds the circulation correction for the given line force model
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
#[derive(Default)]
pub enum CirculationCorrection {
    /// Default, which is no correction.
    #[default]
    None,
    /// The circulation distribution will be set to a prescribed shape, which will be scaled based
    /// on the estimated circulation.
    Prescribed(PrescribedCirculation),
    /// The raw estimated circulation distribution will be smoothed, using either Gaussian or cubic
    /// polynomial smoothing.
    Smoothing(CirculationSmoothing)
}

