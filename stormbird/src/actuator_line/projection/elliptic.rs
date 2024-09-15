// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;

use crate::line_force_model::span_line::SpanLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elliptic {
    pub chord_factor: f64,
    pub thickness_factor: f64,
}

impl Default for Elliptic {
    fn default() -> Self {
        Self {
            chord_factor: 0.4,
            thickness_factor: 0.4,
        }
    }
}

impl Elliptic {
    pub fn projection_value_at_point(&self, point: SpatialVector<3>, chord_vector: SpatialVector<3>, span_line: &SpanLine) -> f64 {
        let chord_length = chord_vector.length();
        let line_length = span_line.length();
        let point_lc = span_line.line_coordinates(point, chord_vector);
        
        let chord_radius     = self.chord_factor * chord_length;
        let thickness_radius = self.thickness_factor * chord_length;

        let chord_term     = (point_lc.chord / chord_radius).powf(2.0);
        let thickness_term = (point_lc.thickness / thickness_radius).powf(2.0);

        let subtract_factor = chord_term + thickness_term;

        let relative_span = point_lc.span / line_length;    

        if subtract_factor > 1.0  || relative_span < -0.5|| relative_span > 0.5 {
            0.0
        } else {
            let integral_factor = 2.0 * PI * chord_radius * thickness_radius / 3.0;
            let span_factor  = 1.0 / line_length;

            (1.0 - subtract_factor).sqrt() * span_factor / integral_factor
        }
    }
}