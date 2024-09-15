// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;

use crate::line_force_model::span_line::SpanLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gaussian {
    pub chord_factor: f64,
    pub thickness_factor: f64,
}

impl Default for Gaussian {
    fn default() -> Self {
        Self {
            chord_factor: 0.4,
            thickness_factor: 0.4,
        }
    }
}

impl Gaussian {
    pub fn projection_value_at_point(&self, point: SpatialVector<3>, chord_vector: SpatialVector<3>, span_line: &SpanLine) -> f64 {
        let chord_length = chord_vector.length();
        let line_length = span_line.length();
        let point_lc = span_line.line_coordinates(point, chord_vector);
        
        let e_chord     = self.chord_factor * chord_length;
        let e_thickness = self.thickness_factor * chord_length;
        
        let chord_term     = (point_lc.chord / e_chord).powi(2);
        let thickness_term = (point_lc.thickness / e_thickness).powi(2);

        let  exp_factor = -chord_term - thickness_term;

        let const_denominator = e_chord * e_thickness * PI;

        let relative_span = point_lc.span / line_length;

        let span_factor = if relative_span > -0.5 && relative_span < 0.5 {
            1.0 / line_length
        } else {
            0.0
        };
        
        (1.0 / const_denominator ) * exp_factor.exp() * span_factor
    }
}