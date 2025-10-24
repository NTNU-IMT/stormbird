// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::spatial_vector::SpatialVector;
use crate::line_force_model::span_line::SpanLine;
use crate::lifting_line::singularity_elements::horseshoe_vortex::HorseshoeVortex;
use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;
use crate::lifting_line::wake::settings::{QuasiSteadyWakeSettings, ViscousCoreLength};

use stormath::matrix::Matrix;
use stormath::type_aliases::Float;

use crate::line_force_model::LineForceModel;

//use rayon::prelude::*;

use crate::lifting_line::wake::dynamic_wake::DynamicWake;

#[derive(Debug, Clone)]
/// Represents a wake where the shape is assumed to be frozen, but where the strength on parts of
/// the wake can be updated. That is, it is intended to be used while solving for the circulation
/// strength in a lifting line simulation. Two primary scenarios exists:
///
/// - The wake is steady, and consists of just one panel/horseshoe vortex per span line, where the
/// strength is unknown
/// - The wake is actually dynamic, but most of the wake consists of panels where the strength is
/// known from previous time steps. The only unknown strength is the strength of the first panels
/// right behind the span lines making up the wings. The induced velocities therefore comes from
/// both the panels with known strength and the panels with unknown strength.
pub struct FrozenWake {
    /// Vector containing values for the induced velocities that are constant for each control point
    /// in the simulation. That is, velocities that do not depend on the circulation strength of the
    /// panels right behind the line model.
    pub fixed_velocities: Vec<SpatialVector>,
    /// Matrix containing coefficients that can be used to calculate induced velocities as a
    /// function of the strength of each vortex line.
    ///
    /// The shape of the matrix is (nr_span_lines, nr_span_lines). Each row corresponds to a control
    /// point. Each column for a given row corresponds to the induced velocity from each panel. The
    /// induced velocity can therefore be calculated as the dot product of the row and the
    /// circulation strength.
    pub variable_velocity_factors: Matrix<SpatialVector>,
    /// Allocate space for the induced velocities at the control points, to avoid reallocating
    /// every time step.
    pub induced_velocities_at_control_points: Vec<SpatialVector>,
}

impl FrozenWake {
    pub fn initialize(nr_span_lines: usize) -> Self {
        let fixed_velocities = vec![SpatialVector::default(); nr_span_lines];

        let variable_velocity_factors = Matrix::new_default(
            [nr_span_lines, nr_span_lines]
        );

        let induced_velocities_at_control_points = vec![SpatialVector::default(); nr_span_lines];

        FrozenWake {
            fixed_velocities,
            variable_velocity_factors,
            induced_velocities_at_control_points,
        }
    }

    /// Function that updates the variable velocity factors assuming a steady wake, given a line
    /// force model and the velocities at the span points.
    pub fn update_as_steady_from_line_force_model_and_velocities(
        &mut self,
        line_force_model: &LineForceModel,
        span_point_velocities: &[SpatialVector],
        wake_settings: &QuasiSteadyWakeSettings,
    ) {
        let nr_span_lines = line_force_model.span_lines_global.len();

        let span_lines = &line_force_model.span_lines_global;

        let average_chord_length = line_force_model.chord_lengths.iter().sum::<Float>() /
            (nr_span_lines as Float);

        let wake_length = wake_settings.wake_length_factor * average_chord_length;

        // Pre-calculate normalized wake vectors to avoid repeated normalization
        let mut wake_vectors: Vec<SpatialVector> = Vec::with_capacity(nr_span_lines + 1);
        for i in 0..=nr_span_lines {
            wake_vectors.push(span_point_velocities[i].normalize() * wake_length);
        }

        let viscous_core_length = match wake_settings.viscous_core_length {
            ViscousCoreLength::Relative(factor) => {
                let average_bound_vortex_length = span_lines.iter()
                    .map(|span_line| span_line.length())
                    .sum::<Float>() / (nr_span_lines as Float);

                factor * average_bound_vortex_length
            },
            ViscousCoreLength::Absolute(length) => length,
            ViscousCoreLength::NoViscousCore => 0.0,
        };

        let horseshoe_vortices = HorseshoeVortex::vortices_from_span_lines_and_wake_vectors(
            &span_lines,
            &wake_vectors,
            viscous_core_length
        );

        for row_index in 0..nr_span_lines {
            let ctrl_point = line_force_model.ctrl_points_global[row_index];

            for col_index in 0..nr_span_lines {
                let vortex = &horseshoe_vortices[col_index];

                let u_i = vortex.induced_velocity_with_unit_strength(ctrl_point);

                let point_mirrored = wake_settings.symmetry_condition.mirrored_point(ctrl_point);

                let u_i_corrected = if let Some(p_m) = point_mirrored {
                    let u_i_mirrored = vortex.induced_velocity_with_unit_strength(p_m);
                    wake_settings.symmetry_condition.corrected_velocity(u_i, u_i_mirrored)
                } else {
                    u_i
                };

                self.variable_velocity_factors[[row_index, col_index]] = u_i_corrected;
            }
        }
    }

    /// Function to create a steady frozen wake from a set of span lines, a wake direction and a
    /// wake length.
    pub fn steady_wake_from_span_lines_and_direction(
        span_lines: &[SpanLine],
        wake_vector: SpatialVector,
        viscous_core_length: Float,
        symmetry_condition: SymmetryCondition
    ) -> Self {
        let nr_span_lines = span_lines.len();

        let ctrl_points: Vec<SpatialVector> = span_lines.iter().map(
            |span_line| span_line.ctrl_point()
        ).collect();

        let fixed_velocities = vec![SpatialVector::default(); nr_span_lines];
        let mut variable_velocity_factors = Matrix::new_default(
            [nr_span_lines, nr_span_lines]
        );

        let wake_vectors = vec![wake_vector; nr_span_lines + 1];

        let horseshoe_vortices = HorseshoeVortex::vortices_from_span_lines_and_wake_vectors(
            &span_lines,
            &wake_vectors,
            viscous_core_length
        );

        for row_index in 0..nr_span_lines {
            let ctrl_point = ctrl_points[row_index];

            for col_index in 0..nr_span_lines {
                let vortex = &horseshoe_vortices[col_index];

                let u_i = vortex.induced_velocity_with_unit_strength(ctrl_point);

                let point_mirrored = symmetry_condition.mirrored_point(ctrl_point);

                let u_i_corrected = if let Some(p_m) = point_mirrored {
                    let u_i_mirrored = vortex.induced_velocity_with_unit_strength(p_m);
                    symmetry_condition.corrected_velocity(u_i, u_i_mirrored)
                } else {
                    u_i
                };

                variable_velocity_factors[[row_index, col_index]] = u_i_corrected;
            }
        }

        let induced_velocities_at_control_points = vec![SpatialVector::default(); nr_span_lines];

        FrozenWake {
            fixed_velocities,
            variable_velocity_factors,
            induced_velocities_at_control_points,
        }

    }

    pub fn update_from_full_wake(&mut self, ctrl_points: &[SpatialVector], wake: &DynamicWake) {
        self.update_fixed_velocities(ctrl_points, wake);
        self.update_variable_velocity_factors(ctrl_points, wake);
    }

    pub fn update_fixed_velocities(&mut self, ctrl_points: &[SpatialVector], wake: &DynamicWake) {
        self.fixed_velocities = wake.induced_velocities_from_free_wake(&ctrl_points);
    }

    fn update_variable_velocity_factors_full(
        &mut self,
        ctrl_points: &[SpatialVector],
        wake: &DynamicWake
    ) {
        let [nr_ctrl_points, nr_panels] = self.variable_velocity_factors.shape;

        for panel_index in 0..nr_panels {
            for ctrl_point_index in 0..nr_ctrl_points {
                let ctrl_point = ctrl_points[ctrl_point_index];

                let induced_velocity = wake.unit_strength_induced_velocity_from_panel(
                    0,
                    panel_index,
                    ctrl_point
                );

                self.variable_velocity_factors[[ctrl_point_index, panel_index]] = induced_velocity;
            }
        }
    }

    fn update_variable_velocity_factors_neglect_self_induced(
        &mut self,
        ctrl_points: &[SpatialVector],
        wake: &DynamicWake
    ) {
        let [nr_ctrl_points, nr_panels] = self.variable_velocity_factors.shape;

        for panel_index in 0..nr_panels {
            for ctrl_point_index in 0..nr_ctrl_points {
                let ctrl_point = ctrl_points[ctrl_point_index];

                let ctrl_point_wing_index = wake.wing_index(ctrl_point_index);
                let panel_wing_index      = wake.wing_index(panel_index);

                if ctrl_point_wing_index == panel_wing_index {
                    self.variable_velocity_factors[[ctrl_point_index, panel_index]] =
                        SpatialVector::default();
                } else {
                    let induced_velocity = wake.unit_strength_induced_velocity_from_panel(
                        0,
                        panel_index,
                        ctrl_point
                    );

                    self.variable_velocity_factors[[ctrl_point_index, panel_index]] =
                        induced_velocity;
                }
            }
        }
    }

    pub fn update_variable_velocity_factors(
        &mut self,
        ctrl_points: &[SpatialVector],
        wake: &DynamicWake
    ) {
        if wake.settings.neglect_self_induced_velocities {
            self.update_variable_velocity_factors_neglect_self_induced(ctrl_points, wake);
        } else {
            self.update_variable_velocity_factors_full(ctrl_points, wake);
        }
    }

    /// Update the stored induced velocity at the control points, given the circulation strength.
    ///
    /// # Arguments
    /// * `circulation_strength` - the circulation strength of the span lines that make up the
    /// wake. The strength is assumed to be constant along the length of the span line.
    pub fn update_induced_velocities_at_control_points(
        &mut self,
        circulation_strength: &[Float],
    ) {
        let nr_rows = self.fixed_velocities.len();
        let nr_cols = self.variable_velocity_factors.shape[1];

        self.induced_velocities_at_control_points.copy_from_slice(&self.fixed_velocities);

        for i_row in 0..nr_rows {
            let row_start = i_row * nr_cols;

            // Separate the components to help auto-vectorization
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut sum_z = 0.0;

            // This pattern is more likely to be auto-vectorized
            for i_col in 0..nr_cols {
                let idx = row_start + i_col;
                let strength = circulation_strength[i_col];
                let factor = &self.variable_velocity_factors.data[idx];

                sum_x += factor.0[0] * strength;
                sum_y += factor.0[1] * strength;
                sum_z += factor.0[2] * strength;
            }

            let result = &mut self.induced_velocities_at_control_points[i_row];
            result.0[0] += sum_x;
            result.0[1] += sum_y;
            result.0[2] += sum_z;
        }
    }
}
