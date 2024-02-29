// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

use serde::{Serialize, Deserialize};

use crate::vec3::Vec3;

use crate::math_utils::interpolation::linear_interpolation;
use crate::math_utils::integration::trapz;

use crate::line_force_model::span_line::SpanLine;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChordVariationFromData {
    pub chord_factor: f64,
    pub thickness_factor: f64,
    pub chord_factor_data: Vec<f64>,
    pub value_data: Vec<f64>,
}

impl ChordVariationFromData {
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

            if relative_chord_coordinate > 1.0 || relative_chord_coordinate < -1.0 {
                0.0
            } else {
                let thickness_value = (-0.5 * relative_thickness_coordinate.powi(2)).exp() / 
                    (thickness_radius * (2.0 * PI).sqrt());

                let integral_value = trapz(&self.chord_factor_data, &self.value_data);

                let value = linear_interpolation(relative_chord_coordinate, &self.chord_factor_data, &self.value_data);

                value * thickness_value / (integral_value * chord_radius * line_length)
            }
        }
    }
}