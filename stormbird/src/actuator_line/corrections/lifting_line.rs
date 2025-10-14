// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::wake::frozen_wake::FrozenWake;

use crate::line_force_model::LineForceModel;

use crate::common_utils::prelude::*;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use serde::{Serialize, Deserialize};

/// A builder for the [LiftingLineCorrection] struct. See the documentation for
/// [LiftingLineCorrection] for more information on how it works.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiftingLineCorrectionBuilder {
    #[serde(default = "LiftingLineCorrectionBuilder::default_wake_length_factor")]
    pub wake_length_factor: Float,
    #[serde(default = "LiftingLineCorrectionBuilder::default_solver_damping_factor")]
    pub solver_damping_factor: Float,
    #[serde(default = "LiftingLineCorrectionBuilder::default_nr_solver_iterations")]
    pub nr_solver_iterations: usize
}

impl LiftingLineCorrectionBuilder {
    fn default_wake_length_factor() -> Float {100.0}
    fn default_solver_damping_factor() -> Float {0.1}
    fn default_nr_solver_iterations() -> usize {20}

    pub fn build(
        &self, 
        viscous_core_length_factor: Float, 
        line_force_model: &LineForceModel
    ) -> LiftingLineCorrection {
        let chord_lengths: Vec<Float> = line_force_model.chord_lengths.clone();

        let average_chord_length = chord_lengths.iter().sum::<Float>() / chord_lengths.len() as Float;

        let viscous_core_length = viscous_core_length_factor * average_chord_length;

        let nr_of_lines = line_force_model.nr_span_lines();

        let velocity_correction_estimate = vec![SpatialVector::default(); nr_of_lines];

        LiftingLineCorrection {
            initialized: false,
            viscous_core_length,
            nr_solver_iterations: self.nr_solver_iterations,
            solver_damping_factor: self.solver_damping_factor,
            velocity_correction_estimate
        }
    }
}

#[derive(Debug, Clone)]
/// A structure used to compute corrections for the velocity, based on a lifting line model.
pub struct LiftingLineCorrection {
    pub initialized: bool,
    pub viscous_core_length: Float,
    pub nr_solver_iterations: usize,
    pub solver_damping_factor: Float,
    velocity_correction_estimate: Vec<SpatialVector>
}

impl LiftingLineCorrection {
    pub fn velocity_correction(
        &mut self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        let span_lines = &line_force_model.span_lines_global;

        let mut u_i_correction: Vec<SpatialVector> = Vec::with_capacity(span_lines.len());

        for wing_index in 0..line_force_model.nr_wings() {
            let wind_indices = line_force_model.wing_indices[wing_index].clone();

            let wing_span_lines = &span_lines[
                wind_indices.clone()
            ];

            let wing_circulation_strength = &circulation_strength[
                wind_indices.clone()
            ];

            let nr_span_lines = wing_span_lines.len();

            let wing_ctrl_points_velocity = &ctrl_points_velocity[
                wind_indices.clone()
            ];

            let averaged_ctrl_points_velocity = wing_ctrl_points_velocity.iter().sum::<SpatialVector>() 
                / wing_ctrl_points_velocity.len() as Float;

            let wake_vector = averaged_ctrl_points_velocity.normalize() * 100.0;

            let far_field_ratio = 5.0; // Ratio of far field length to viscous core length

            let mut frozen_wake_viscous = FrozenWake::steady_wake_from_span_lines_and_direction(
                wing_span_lines,
                wake_vector,
                self.viscous_core_length,
                far_field_ratio, // far_field_ratio
            );

            let mut frozen_wake_default = FrozenWake::steady_wake_from_span_lines_and_direction(
                wing_span_lines,
                wake_vector,
                self.viscous_core_length / 10.0,
                far_field_ratio, // far_field_ratio
            );

            frozen_wake_viscous.update_induced_velocities_at_control_points(
                &wing_circulation_strength
            );

            frozen_wake_default.update_induced_velocities_at_control_points(
                &wing_circulation_strength
            );

            let u_i_viscous = &frozen_wake_viscous.induced_velocities_at_control_points;
            let u_i_default = &frozen_wake_default.induced_velocities_at_control_points;

            for i in 0..nr_span_lines {
                u_i_correction.push(u_i_default[i] - u_i_viscous[i]);
            }
        }
        
        u_i_correction   
    }

    pub fn solve_correction(
        &mut self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        circulation_strength: &[Float],
    ) -> (
        Vec<SpatialVector>,
        Vec<Float>
    ) {
        let mut corrected_ctrl_points_velocity = ctrl_points_velocity.to_vec();
        let mut corrected_circulation_strength = circulation_strength.to_vec();

        for _ in 0..self.nr_solver_iterations {
            let new_velocity_correction_estimate: Vec<SpatialVector> = self.velocity_correction(
                line_force_model,
                ctrl_points_velocity,
                &corrected_circulation_strength,
            );

            for j in 0..corrected_ctrl_points_velocity.len() {
                let current_velocity_correction = self.velocity_correction_estimate[j];

                self.velocity_correction_estimate[j] += (
                    new_velocity_correction_estimate[j] - current_velocity_correction
                ) * self.solver_damping_factor;

                corrected_ctrl_points_velocity[j] = ctrl_points_velocity[j] + self.velocity_correction_estimate[j];
            }

            corrected_circulation_strength = line_force_model.circulation_strength(
                &corrected_ctrl_points_velocity, 
                CoordinateSystem::Global
            );
        }

        (corrected_ctrl_points_velocity, corrected_circulation_strength)
    }
}