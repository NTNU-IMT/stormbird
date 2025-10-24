// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::wake::frozen_wake::FrozenWake;
use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;

use crate::line_force_model::LineForceModel;

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
    #[serde(default)]
    pub symmetry_condition: SymmetryCondition
}

impl LiftingLineCorrectionBuilder {
    fn default_wake_length_factor() -> Float {100.0}

    pub fn build(
        &self,
        viscous_core_length_factor: Float,
        line_force_model: &LineForceModel
    ) -> LiftingLineCorrection {
        let chord_lengths: Vec<Float> = line_force_model.chord_lengths.clone();

        let average_chord_length = chord_lengths.iter().sum::<Float>() / chord_lengths.len() as Float;

        let viscous_core_length = viscous_core_length_factor * average_chord_length;

        LiftingLineCorrection {
            viscous_core_length,
            wake_length_factor: self.wake_length_factor,
            symmetry_condition: self.symmetry_condition
        }
    }
}

#[derive(Debug, Clone)]
/// A structure used to compute corrections for the velocity, based on a lifting line model. The
/// model computes two versions of induced velocities using a lifting line representation; one where
/// the viscous core length is set to a small value, and one where it is set to a value equal to the
/// force projection width. The difference in induced velocities are then added to the velocity that
/// is sampled directly from the flow field. This is to account for the fact that realistic force
/// projection widths tend to smear out the tip vortices, leading to under predicted induced
/// velocities
pub struct LiftingLineCorrection {
    pub viscous_core_length: Float,
    pub wake_length_factor: Float,
    pub symmetry_condition: SymmetryCondition
}

impl LiftingLineCorrection {
    /// Computes a difference in the induced velocities between two lifting line simualtions; one
    /// with a viscous core length equal to the force projection width, and one with a small viscous
    /// core length. The difference is then returned as a vector of induced velocities at the control
    /// points of the line force model
    pub fn velocity_correction(
        &self,
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

            let wake_vector = averaged_ctrl_points_velocity.normalize() * self.wake_length_factor;

            let mut frozen_wake_viscous = FrozenWake::steady_wake_from_span_lines_and_direction(
                wing_span_lines,
                wake_vector,
                self.viscous_core_length,
                self.symmetry_condition
            );

            let mut frozen_wake_default = FrozenWake::steady_wake_from_span_lines_and_direction(
                wing_span_lines,
                wake_vector,
                self.viscous_core_length / 10.0,
                self.symmetry_condition
            );

            frozen_wake_viscous.update_induced_velocities_at_control_points(
                &wing_circulation_strength
            );

            frozen_wake_default.update_induced_velocities_at_control_points(
                &wing_circulation_strength
            );

            for i in 0..nr_span_lines {
                u_i_correction.push(
                    frozen_wake_default.induced_velocities_at_control_points[i] -
                    frozen_wake_viscous.induced_velocities_at_control_points[i]
                );
            }
        }

        u_i_correction
    }
}
