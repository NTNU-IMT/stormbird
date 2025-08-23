// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

use crate::line_force_model::span_line::SpanLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gaussian {
    pub chord_factor: Float,
    pub thickness_factor: Float,
}

impl Default for Gaussian {
    fn default() -> Self {
        Self {
            chord_factor: 0.4,
            thickness_factor: 0.2
        }
    }
}

impl Gaussian {
    pub fn projection_value_at_point(
        &self, 
        point: SpatialVector, 
        chord_vector: SpatialVector, 
        span_line: &SpanLine
    ) -> Float {
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