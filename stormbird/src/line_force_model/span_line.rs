// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;

#[derive(Debug, Clone)]
/// Struct that represents spatial coordinates in a coordinate system that is oriented along the 
/// chord line, span line and thickness direction of a line segment.
pub struct LineCoordinates {
    pub chord: f64,
    pub thickness: f64,
    pub span: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// A line segement of a wing span
pub struct SpanLine {
    pub start_point: SpatialVector<3>,
    pub end_point: SpatialVector<3>
}

impl SpanLine {
    pub fn translate(&self, translation: SpatialVector<3>) -> Self {
        Self {
            start_point: self.start_point + translation,
            end_point: self.end_point + translation
        }
    }

    pub fn rotate(&self, rotation: SpatialVector<3>) -> Self {
        Self {
            start_point: self.start_point.rotate(rotation),
            end_point: self.end_point.rotate(rotation)
        }
    }

    pub fn rotate_around_axis(&self, angle: f64, axis: SpatialVector<3>) -> Self {
        Self {
            start_point: self.start_point.rotate_around_axis(angle, axis),
            end_point: self.end_point.rotate_around_axis(angle, axis)
        }
    }

    pub fn relative_vector(&self) -> SpatialVector<3> {
        self.end_point - self.start_point
    }

    pub fn length(&self) -> f64 {
        self.relative_vector().length()
    }

    pub fn direction(&self) -> SpatialVector<3> {
        self.relative_vector().normalize()
    }

    pub fn as_array(&self) -> [SpatialVector<3>; 2] {
        [self.start_point, self.end_point]
    }

    /// Return the control point of the line segment, which corresponds to the average point along 
    /// the line segment. 
    pub fn ctrl_point(&self) -> SpatialVector<3> {
        0.5 * (self.start_point + self.end_point)
    }

    pub fn distance(&self, point: SpatialVector<3>) -> f64 {
        let relative_vector = self.relative_vector();
        let start_to_point  = point - self.start_point;
        let end_to_point    = point - self.end_point;

        if start_to_point.dot(relative_vector) <= 0.0 {
            start_to_point.length()
        } else if end_to_point.dot(relative_vector) >= 0.0 {
            end_to_point.length()
        } else {
            relative_vector.cross(start_to_point).length() / relative_vector.length()
        }
    }

    /// Computes the line coordinates of the input point, based on the geoemtry of Self, and an 
    /// input chord vector.
    /// 
    /// The chord and span direction is given directly by Self and the input. The thickness 
    /// direction is assumed to be normal to the two other directions.
    pub fn line_coordinates(&self, point: SpatialVector<3>, chord_vector: SpatialVector<3>) -> LineCoordinates {
        let translated_point = point - self.ctrl_point();

        let span_direction      = self.relative_vector().normalize();
        let chord_direction     = chord_vector.normalize();
        let thickness_direction = span_direction.cross(chord_direction);

        LineCoordinates {
            chord:     translated_point.dot(chord_direction),
            thickness: translated_point.dot(thickness_direction),
            span:      translated_point.dot(span_direction)
        }
    }
}