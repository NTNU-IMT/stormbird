// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use crate::line_force_model::LineForceModel;
use crate::section_models::SectionModel;

use stormath::smoothing;
use stormath::smoothing::end_condition::EndCondition;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GaussianSmoothing {
    #[serde(default="GaussianSmoothing::default_length_factor")]
    /// A non dimensional factor used to calculate the length in Gaussian smoothing kernel. 
    /// The actual smoothing length is calculated as the length factor times the wing span.
    pub length_factor: f64,
    #[serde(default)]
    /// Option to only do interior smoothing.
    pub number_of_end_points_to_interpolate: usize,
}

impl GaussianSmoothing {
    fn default_length_factor() -> f64 {0.1}
}

impl Default for GaussianSmoothing {
    fn default() -> Self {
        Self {
            length_factor: Self::default_length_factor(),
            number_of_end_points_to_interpolate: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ValueTypeToBeSmoothed {
    Circulation,
    AngleOfAttack,
}

