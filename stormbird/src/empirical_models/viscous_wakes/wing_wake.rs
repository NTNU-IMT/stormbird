// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::vec3::Vec3;
use crate::line_force_model::span_line::SpanLine;

use super::wake_models;

use super::*;

#[derive(Debug, Clone)]
pub struct WingWake {
    pub span_line: SpanLine,
    pub chord_vector: Vec3,
}

impl WingWake {
    /// Checks and computes the wake state of a point, based on the current velocity. If the point is 
    /// in the wake, the function returns the wake coordinates.If it is not in the wake, it returns
    /// the `WakeState::NotInWake` variant.
    /// 
    /// The check is based on whether or not the point is in the "shadow" of the span line. 
    pub fn wake_state_of_point(&self, point: Vec3, velocity: Vec3) -> WakeState {
        let ctrl_point = self.span_line.ctrl_point();

        let relative_projected_span = self.span_line.relative_vector().project_on_plane(velocity);

        let projected_span = SpanLine{
            start_point: ctrl_point - 0.5 * relative_projected_span,
            end_point: ctrl_point + 0.5 * relative_projected_span,
        };

        let normal_direction = velocity.cross(relative_projected_span).normalize();

        let projected_span_coordinates = projected_span.line_coordinates(point, self.chord_vector);

        let projected_span_length = projected_span.length();

        let relative_point = point - ctrl_point;

        let relative_projected_point = relative_point.project(velocity);

        let velocity_coordinate = if relative_projected_point.absolute_angle_between(velocity) < 1e-6 {
            relative_projected_point.length()
        } else {
            -relative_projected_point.length()  
        };

        let span_test = projected_span_coordinates.span >= -0.5 * projected_span_length && projected_span_coordinates.span <= 0.5 * projected_span_length;
        let velocity_test = velocity_coordinate > 0.0;

        if span_test && velocity_test {
            WakeState::InWake(
                WakeCoordinates{
                    velocity: velocity_coordinate,
                    normal: relative_point.project(normal_direction).length(),
                }
            )
        } else {
            WakeState::NotInWake
        }
    }

    pub fn velocity_correction(&self, point: Vec3, velocity: Vec3, drag_coefficient: f64) -> f64 {
        let wake_state = self.wake_state_of_point(point, velocity);

        let model_parameters = WakeModelParameters{
            drag_coefficient,
            width: self.chord_vector.project_on_plane(velocity).length(),
        };

        let correction = match &wake_state {
            WakeState::InWake(wake_coordinates) => wake_models::schlichting_correction_factor(&wake_coordinates, &model_parameters),
            WakeState::NotInWake => 1.0,
        };

        correction
    }
}