// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use crate::lifting_line::prelude::SolverResult;
use crate::line_force_model::LineForceModel;
use crate::lifting_line::wake::frozen_wake::FrozenWake;
use crate::common_utils::forces_and_moments::CoordinateSystem;

use stormath::spatial_vector::SpatialVector;
use stormath::matrix::Matrix;

use super::velocity_corrections::VelocityCorrections;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Linearized {
    #[serde(default)]
    pub velocity_corrections: VelocityCorrections, // TODO: use this in the solver...
    #[serde(default)]
    pub disable_viscous_corrections: bool,
}

impl Default for Linearized {
    fn default() -> Self {
        Linearized {
            velocity_corrections: VelocityCorrections::default(),
            disable_viscous_corrections: false
        }
    }
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

        let mut velocity_with_fixed_induced = felt_ctrl_points_freestream.to_vec();

        for i in 0..nr_unknowns {
            velocity_with_fixed_induced[i] += frozen_wake.fixed_velocities[i];
        }

        // Compute relevant directions
        let mut velocity_dir = Vec::with_capacity(nr_unknowns);
        let mut normal_dir = Vec::with_capacity(nr_unknowns);

        for i in 0..nr_unknowns {
            let u_norm = velocity_with_fixed_induced[i].normalize();

            let span_dir = line_force_model.span_lines_global[i].relative_vector().normalize();

            velocity_dir.push(u_norm);
            normal_dir.push(
                u_norm.cross(span_dir)
            );
        }

        let angles_of_attack = line_force_model.angles_of_attack(
            &velocity_with_fixed_induced, 
            CoordinateSystem::Global
        );

        let cl_linearized = line_force_model.linearized_lift_coefficients(&angles_of_attack);

        // Extract lift derivatives
        let dcl_dalpha = line_force_model.lift_coefficient_derivatives();

        // Create equation system
        let mut equation_matrix = Matrix::new_default([nr_unknowns, nr_unknowns]);
        let mut rhs = vec![0.0; nr_unknowns];

        for i_row in 0..nr_unknowns {
            let u_t = velocity_with_fixed_induced[i_row].dot(velocity_dir[i_row]);

            for i_col in 0..nr_unknowns {
                let u_i_n = frozen_wake.variable_velocity_factors[[i_row, i_col]].dot(normal_dir[i_row]);

                // Lift at ctrl_point i_row due to induced angle of attack from line i_col
                equation_matrix[[i_row, i_col]] = -dcl_dalpha[i_row] * u_i_n / u_t;

                if i_row == i_col {
                    // Lift due to circulation
                    equation_matrix[[i_row, i_col]] += 2.0 / ( line_force_model.chord_lengths[i_row] * u_t);
                }
            }

            rhs[i_row] = -cl_linearized[i_row];
        }

        // Solve for the circulation strength

        let mut circulation_strength = equation_matrix.solve_gaussian_elimination(&rhs).unwrap();

        frozen_wake.update_induced_velocities_at_control_points(&circulation_strength);

        let mut ctrl_point_velocity = Vec::with_capacity(nr_unknowns);

        for i in 0..nr_unknowns {
            ctrl_point_velocity.push(
                felt_ctrl_points_freestream[i] + frozen_wake.induced_velocities_at_control_points[i]
            );
        }

        // Calculate corrections on the circulation due to viscous effects
        if !self.disable_viscous_corrections {
            let angles_of_attack = line_force_model.angles_of_attack(
                &ctrl_point_velocity, 
                CoordinateSystem::Global
            );

            let cl_linearized = line_force_model.linearized_lift_coefficients(&angles_of_attack);

            let cl_full = line_force_model.lift_coefficients(
                &ctrl_point_velocity, 
                CoordinateSystem::Global
            );

            for i in 0..circulation_strength.len() {
                circulation_strength[i] *= cl_full[i] / cl_linearized[i]
            }
        }


        let residual = line_force_model.average_residual_absolute(
            &circulation_strength, 
            &ctrl_point_velocity,
            CoordinateSystem::Global
        );

        SolverResult {
            input_ctrl_point_velocity: felt_ctrl_points_freestream.to_vec(),
            circulation_strength,
            output_ctrl_point_velocity: ctrl_point_velocity,
            iterations: 1,
            residual
        }
    }
}
