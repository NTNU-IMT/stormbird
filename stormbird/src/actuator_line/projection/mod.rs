// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use crate::line_force_model::span_line::SpanLine;

pub mod gaussian;
pub mod elliptic;
pub mod harmonic;
pub mod chord_variation_from_data;

use gaussian::Gaussian;
use elliptic::Elliptic;
use harmonic::Harmonic;
use chord_variation_from_data::ChordVariationFromData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Projection {
    Gaussian(Gaussian),
    Elliptic(Elliptic),
    Harmonic(Harmonic),
    ChordVariationFromData(ChordVariationFromData),
}

impl Default for Projection {
    fn default() -> Self {
        Self::Gaussian(Gaussian::default())
    }
}


impl Projection {
    pub fn projection_value_at_point(&self, point: SpatialVector<3>, chord_vector: SpatialVector<3>, span_line: &SpanLine) -> f64 {
        match self {
            Self::Gaussian(gaussian) => gaussian.clone().projection_value_at_point(point, chord_vector, span_line),
            Self::Elliptic(elliptic) => elliptic.clone().projection_value_at_point(point, chord_vector, span_line),
            Self::Harmonic(harmonic) => harmonic.clone().projection_value_at_point(point, chord_vector, span_line),
            Self::ChordVariationFromData(chord_variation_from_data) => chord_variation_from_data.clone().projection_value_at_point(point, chord_vector, span_line),
        }
    }
}