// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Empirical models of viscous wakes that occur when the flow separates from wings.

pub mod wing_wake;
pub mod wake_models;

use crate::stormath::statistics::mean;
use crate::vec3::Vec3;

use wing_wake::WingWake;
use crate::line_force_model::{LineForceModel, span_line::SpanLine};

#[derive(Debug, Clone)]
pub struct WakeCoordinates {
    pub velocity: f64,
    pub normal: f64,
}

#[derive(Debug, Clone)]
pub struct WakeModelParameters {
    pub drag_coefficient: f64,
    pub width: f64,
}

#[derive(Debug, Clone)]
pub enum WakeState {
    InWake(WakeCoordinates),
    NotInWake,
}

pub struct ViscousWakes {
    pub wing_wakes: Vec<WingWake>,
}

impl ViscousWakes {
    pub fn from_line_force_model(line_force_model: &LineForceModel) -> Self {
        let mut wing_wakes: Vec<WingWake> = Vec::new();
        let span_lines_global = line_force_model.span_lines();
        let chord_vectors_global = line_force_model.chord_vectors();

        for wing_indices in &line_force_model.wing_indices {
            let chord_vectors = &chord_vectors_global[wing_indices.clone()];
            let span_lines = &span_lines_global[wing_indices.clone()];

            let effective_span_line = SpanLine{
                start_point: span_lines.first().unwrap().start_point,
                end_point: span_lines.last().unwrap().end_point,
            };

            wing_wakes.push(
                WingWake{
                    chord_vector: mean(chord_vectors),
                    span_line: effective_span_line,
                }
            );
        }

        Self{
            wing_wakes,
        }
    }

    pub fn velocity_correction(&self, point: Vec3, velocities: &[Vec3], drag_coefficients: &[f64]) -> f64 {
        self.wing_wakes.iter().zip(velocities.iter().zip(drag_coefficients.iter())).map(
            |(wake, (velocity, cd))| {
                wake.velocity_correction(point, *velocity, *cd)
            }
        ).product()
    }

    pub fn velocity_corrections(&self, points: &[Vec3], velocities: &[Vec3], drag_coefficients: &[f64]) -> Vec<f64> {
        points.iter().map(
            |point| self.velocity_correction(*point, velocities, drag_coefficients)
        ).collect()
    }

    pub fn corrected_velocity(&self, force_model: &LineForceModel, velocity: &[Vec3]) -> Vec<Vec3> {
        let mut velocity_out = velocity.to_vec();

        let ctrl_points               = force_model.ctrl_points();
        let viscous_drag_coefficients = force_model.average_viscous_drag_coefficients(&velocity);
        let wing_averaged_velocity    = force_model.wing_averaged_values(&velocity);

        let velocity_correction: Vec<f64> = self.velocity_corrections(
            &ctrl_points, 
            &wing_averaged_velocity, 
            &viscous_drag_coefficients
        );

        for i in 0..velocity_out.len() {
            velocity_out[i] *= velocity_correction[i];
        }

        velocity_out
    }
}






