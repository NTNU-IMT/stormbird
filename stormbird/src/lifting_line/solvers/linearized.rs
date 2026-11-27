// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};
use stormath::type_aliases::Float;
use stormath::consts::MIN_POSITIVE;

use crate::lifting_line::prelude::SolverResult;
use crate::line_force_model::LineForceModel;
use crate::lifting_line::wake::frozen_wake::FrozenWake;
use crate::common_utils::forces_and_moments::CoordinateSystem;

use stormath::spatial_vector::SpatialVector;
use stormath::matrix::Matrix;

use super::velocity_corrections::InducedVelocitySettings;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// How the induced velocities are updated after the viscous correction of the circulation
pub enum InducedVelocityCorrectionMethod {
    /// The induced velocities from the linear solution are used. The circulation is then corrected
    /// with the same angles of attack as the forces are calculated with, so that the correction is
    /// consistent with the post-stall lift in the force calculation.
    #[default]
    NoCorrection,
    /// The induced velocities are computed again from the corrected circulation. The correction of
    /// the circulation is then based on different angles of attack than the forces, which can give
    /// both unstalled circulation and post-stall lift right before stall, seen as a jump in the
    /// lift.
    FullCorrection
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Structure with settings for a linearized lifting line solver
#[derive(Default)]
pub struct Linearized {
    /// How the induced velocities are used, including optional empirical corrections
    #[serde(default)]
    pub induced_velocities: InducedVelocitySettings,
    /// If true, the circulation is not corrected for non-linear effects, such as stall
    #[serde(default)]
    pub disable_viscous_corrections: bool,
    #[serde(default)]
    /// How the induced velocities are updated after the viscous correction of the circulation
    pub induced_velocity_correction_method: InducedVelocityCorrectionMethod
}


impl Linearized {

    /// Solves for the circulation strength assuming that there is a linear relationship between the
    /// induced velocity and the circulation strength, as was the case for the original lifting line
    /// theory.
    pub fn solve(
        &self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector],
        frozen_wake: &mut FrozenWake
    ) -> SolverResult {

        let nr_unknowns = line_force_model.nr_span_lines();

        let mut ctrl_points_velocity = felt_ctrl_points_freestream.to_vec();

        // Add fixed velocities from the frozen wake
        for (i, velocity) in ctrl_points_velocity.iter_mut().enumerate().take(nr_unknowns) {
            *velocity += self.induced_velocities.uncorrected_induced_velocity(
                frozen_wake.fixed_velocities.self_induced[i],
                frozen_wake.fixed_velocities.other_induced[i]
            );
        }

        ctrl_points_velocity = line_force_model.remove_span_velocity(
            &ctrl_points_velocity,
            CoordinateSystem::Global
        );

        // Compute relevant directions
        let mut velocity_dir = Vec::with_capacity(nr_unknowns);
        let mut normal_dir = Vec::with_capacity(nr_unknowns);

        for (i, velocity) in ctrl_points_velocity.iter().enumerate().take(nr_unknowns) {
            let u_norm = velocity.normalize();

            let span_dir = line_force_model.span_lines_global[i].relative_vector().normalize();

            velocity_dir.push(u_norm);
            normal_dir.push(
                u_norm.cross(span_dir)
            );
        }

        // Compute lift coefficients
        let angles_of_attack = line_force_model.angles_of_attack(
            &ctrl_points_velocity,
            CoordinateSystem::Global
        );

        let internal_states = line_force_model.section_models_internal_state(&ctrl_points_velocity);

        // The linear lift uses the same internal state as the full lift, i.e., the lagged internal
        // state for wings with a dynamic stall model
        let dynamic_internal_states = line_force_model.dynamic_internal_states(
            &angles_of_attack,
            &internal_states,
            &ctrl_points_velocity
        );

        let cl_linear = line_force_model.lift_coefficients_linear(
            &angles_of_attack,
            &dynamic_internal_states,
        );

        // Extract lift derivatives
        let dcl_dalpha = line_force_model.lift_coefficients_derivatives(&dynamic_internal_states);

        // Create equation system
        let mut equation_matrix = Matrix::new_default([nr_unknowns, nr_unknowns]);
        let mut rhs = vec![0.0; nr_unknowns];

        let neglect_self_induced = self.induced_velocities.neglect_self_induced_velocities;

        for wing_range in &frozen_wake.wing_indices {
            // The columns of the same wing as the rows are skipped if the self-induced velocities
            // are neglected. The equation matrix is then zero there, except for the diagonal below.
            let skipped_columns = if neglect_self_induced {
                wing_range.clone()
            } else {
                nr_unknowns..nr_unknowns
            };

            for i_row in wing_range.clone() {
                let u_t = ctrl_points_velocity[i_row].dot(velocity_dir[i_row]);

                // No velocity gives no circulation. The row is otherwise singular, as the lift
                // from the circulation and the induced angle of attack are divided by the velocity
                if u_t < MIN_POSITIVE {
                    equation_matrix[[i_row, i_row]] = 1.0;
                    rhs[i_row] = 0.0;

                    continue;
                }

                let columns = (0..skipped_columns.start).chain(skipped_columns.end..nr_unknowns);

                for i_col in columns {
                    let u_i_n = frozen_wake.variable_velocity_factors[[i_row, i_col]]
                        .dot(normal_dir[i_row]);

                    // Lift at ctrl_point i_row due to induced angle of attack from line i_col
                    equation_matrix[[i_row, i_col]] = -dcl_dalpha[i_row] * u_i_n / u_t;
                }

                // Lift due to circulation
                equation_matrix[[i_row, i_row]] += 2.0 /
                    ( line_force_model.chord_lengths[i_row] * u_t);

                rhs[i_row] = -cl_linear[i_row];
            }
        }

        // Solve for the circulation strength
        let mut circulation_strength = equation_matrix.solve_gaussian_elimination(&rhs).expect(
            "The equation system in the linearized solver is singular. This can happen if the             chord length or the tangential velocity is zero for one or more line elements"
        );

        frozen_wake.update_induced_velocities_at_control_points(&circulation_strength);

        ctrl_points_velocity = self.ctrl_points_velocity_from_solution(
            line_force_model,
            felt_ctrl_points_freestream,
            frozen_wake
        );

        // Calculate corrections on the circulation due to viscous effects
        if !self.disable_viscous_corrections {
            let angles_of_attack = line_force_model.angles_of_attack(
                &ctrl_points_velocity,
                CoordinateSystem::Global
            );

            let internal_states = line_force_model.section_models_internal_state(&ctrl_points_velocity);

            let dynamic_internal_states = line_force_model.dynamic_internal_states(
                &angles_of_attack,
                &internal_states,
                &ctrl_points_velocity
            );

            let cl_linear = line_force_model.lift_coefficients_linear(
                &angles_of_attack,
                &dynamic_internal_states,
            );

            // The same part of the lift as the circulation should correspond to, i.e., including
            // the post-stall lift if it is included in the circulation
            let cl_full = line_force_model.lift_coefficients_for_circulation(
                &angles_of_attack,
                &internal_states,
                &ctrl_points_velocity
            );

            for i in 0..circulation_strength.len() {
                if cl_linear[i].abs() > Float::MIN_POSITIVE {
                    circulation_strength[i] *= cl_full[i] / cl_linear[i]
                }
            }

            match &self.induced_velocity_correction_method {
                InducedVelocityCorrectionMethod::NoCorrection => {},
                InducedVelocityCorrectionMethod::FullCorrection => {
                    frozen_wake.update_induced_velocities_at_control_points(&circulation_strength);
                }
            }

            ctrl_points_velocity = self.ctrl_points_velocity_from_solution(
                line_force_model,
                felt_ctrl_points_freestream,
                frozen_wake
            );
        }

        let angles_of_attack = line_force_model.angles_of_attack(
            &ctrl_points_velocity,
            CoordinateSystem::Global
        );

        let internal_states = line_force_model.section_models_internal_state(&ctrl_points_velocity);

        let residual = line_force_model.average_residual_absolute(
            &circulation_strength,
            &angles_of_attack,
            &internal_states,
            &ctrl_points_velocity
        );

        SolverResult {
            input_ctrl_points_velocity: felt_ctrl_points_freestream.to_vec(),
            circulation_strength,
            output_ctrl_points_velocity: ctrl_points_velocity,
            iterations: 1,
            residual
        }
    }

    /// The velocity at the control points, with the induced velocities from the current
    /// circulation in the frozen wake. The velocity along the span is removed, in the same way as
    /// before the linear system is set up, and as in the iterative solver.
    fn ctrl_points_velocity_from_solution(
        &self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector],
        frozen_wake: &FrozenWake
    ) -> Vec<SpatialVector> {
        let ctrl_points_velocity = self.induced_velocities.ctrl_points_velocity(
            felt_ctrl_points_freestream,
            &frozen_wake.induced_velocities_at_control_points
        );

        line_force_model.remove_span_velocity(&ctrl_points_velocity, CoordinateSystem::Global)
    }
}
