// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub mod prescribed_circulation;
pub mod smoothing;

use serde::{Serialize, Deserialize};

use prescribed_circulation::PrescribedCirculationShape;
use smoothing::GaussianSmoothing;


#[derive(Debug, Clone, Serialize, Deserialize)]
/// Enum for controlling what type of correction to apply to the estimated circulation distribution.
pub enum CirculationCorrection {
    /// Default, which is no correction.
    None,
    /// The circulation distribution will be set to a prescribed shape, which will be scaled based
    /// on the estimated circulation.
    PrescribedCirculation(PrescribedCirculationShape),
    /// The raw estimated circulation distribution will be smoothed using a Gaussian kernel.
    GaussianSmoothing(GaussianSmoothing),
    /// Polynomial smoothing of the estimated circulation distribution.
    PolynomialSmoothing,
}

impl Default for CirculationCorrection {
    fn default() -> Self {
        CirculationCorrection::None
    }
}

