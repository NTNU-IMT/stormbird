// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

use serde::{Serialize, Deserialize};

use crate::vec3::Vec3;

use crate::line_force_model::span_line::SpanLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Harmonic {
    pub chord_factor: f64,
    pub thickness_factor: f64,
}

impl Default for Harmonic {
    fn default() -> Self {
        Self {
            chord_factor: 0.4,
            thickness_factor: 0.4,
        }
    }
}

impl Harmonic {
    pub fn projection_value_at_point(&self, point: Vec3, chord_vector: Vec3, span_line: &SpanLine) -> f64 {
        let line_length = span_line.length();
        let point_lc = span_line.line_coordinates(point, chord_vector);

        let relative_span = point_lc.span / line_length;

        if relative_span < -0.5 || relative_span > 0.5 {
            0.0
        } else {
            let chord_length = chord_vector.length();

            let chord_radius     = self.chord_factor * chord_length;
            let thickness_radius = self.thickness_factor * chord_length;

            let relative_chord_coordinate     = point_lc.chord / chord_radius;
            let relative_thickness_coordinate = point_lc.thickness / thickness_radius;

            let relative_coordinate = (relative_chord_coordinate.powi(2) + relative_thickness_coordinate.powi(2)).sqrt();

            if relative_coordinate > 1.0 {
                0.0
            } else {
                let int_factor = 4.0 - 8.0 / PI; // From Wolfram alpha

                (relative_coordinate * PI * 0.5).cos() / (line_length * int_factor * chord_radius * thickness_radius)
            }
        }
    }
}