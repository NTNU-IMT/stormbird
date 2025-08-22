// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use crate::line_force_model::span_line::SpanLine;

pub mod gaussian;

use gaussian::Gaussian;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionSettings {
    #[serde(default)]
    pub projection_function: Gaussian,
    #[serde(default)]
    pub project_normal_to_velocity: bool,
    #[serde(default="ProjectionSettings::default_weight_limit")]
    pub weight_limit: f64,
    #[serde(default)]
    pub project_sectional_drag: bool,
}

impl Default for ProjectionSettings {
    fn default() -> Self {
        Self {
            projection_function: Gaussian::default(),
            project_normal_to_velocity: false,
            weight_limit: Self::default_weight_limit(),
            project_sectional_drag: false,
        }
    }
}


impl ProjectionSettings {
    fn default_weight_limit() -> f64 {0.001}

    pub fn projection_value_at_point(
        &self, 
        point: SpatialVector, 
        chord_vector: SpatialVector, 
        span_line: &SpanLine
    ) -> f64 {
        self.projection_function.projection_value_at_point(
            point, chord_vector, span_line
        )
    }
}