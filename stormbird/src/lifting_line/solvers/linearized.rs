// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Deserialize, Serialize};

use crate::lifting_line::prelude::SolverResult;
use crate::line_force_model::LineForceModel;
use crate::lifting_line::wake::frozen_wake::FrozenWake;
use crate::common_utils::forces_and_moments::CoordinateSystem;

use stormath::spatial_vector::SpatialVector;
use stormath::matrix::Matrix;

use super::velocity_corrections::VelocityCorrections;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum FinalCirculationCorrectionMethod {
    NoCorrection,
    #[default]
    IncludeViscousLiftInEffectiveCirculation,
    FullCorrection
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Linearized {
    #[serde(default)]
    pub velocity_corrections: VelocityCorrections,
    #[serde(default)]
    pub disable_viscous_corrections: bool,
    #[serde(default)]
    pub final_circulation_correction_method: FinalCirculationCorrectionMethod
}

impl Default for Linearized {
    fn default() -> Self {
        Linearized {
            velocity_corrections: VelocityCorrections::default(),
            disable_viscous_corrections: false,
            final_circulation_correction_method: FinalCirculationCorrectionMethod::default()
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

        let mut ctrl_point_velocity = felt_ctrl_points_freestream.to_vec();

        // Compute relevant directions
        let mut velocity_dir = Vec::with_capacity(nr_unknowns);
        let mut normal_dir = Vec::with_capacity(nr_unknowns);

        for i in 0..nr_unknowns {
            let u_norm = ctrl_point_velocity[i].normalize();

            let span_dir = line_force_model.span_lines_global[i].relative_vector().normalize();

            velocity_dir.push(u_norm);
            normal_dir.push(
                u_norm.cross(span_dir)
            );
        }

        // Add fixed velocities from the frozen wake
        for i in 0..nr_unknowns {
            ctrl_point_velocity[i] += frozen_wake.fixed_velocities[i];
        }

        // Check if there is any lift at all
        let mut angles_of_attack = line_force_model.angles_of_attack(
            &ctrl_point_velocity,
            CoordinateSystem::Global
        );

        let cl_linear_pre_solver = line_force_model.lift_coefficients_linear(
            &angles_of_attack,
            &ctrl_point_velocity
        );

        let mut cl_linear_pre_solver_is_large = true;

        for i in 0..cl_linear_pre_solver.len() {
            if cl_linear_pre_solver[i].abs() > 1e-9 {
                cl_linear_pre_solver_is_large = true;
                break;
            }
        }

        if !cl_linear_pre_solver_is_large {
            return SolverResult {
                input_ctrl_point_velocity: felt_ctrl_points_freestream.to_vec(),
                circulation_strength: vec![0.0; nr_unknowns],
                output_ctrl_point_velocity: felt_ctrl_points_freestream.to_vec(),
                iterations: 0,
                residual: 0.0
            };
        }

        // Compute lift coefficients
        let cl_linear = line_force_model.lift_coefficients_linear(
            &angles_of_attack, 
            &ctrl_point_velocity
        );

        // Extract lift derivatives
        let dcl_dalpha = line_force_model.lift_coefficients_derivatives();

        // Create equation system
        let mut equation_matrix = Matrix::new_default([nr_unknowns, nr_unknowns]);
        let mut rhs = vec![0.0; nr_unknowns];

        for i_row in 0..nr_unknowns {
            let u_t = ctrl_point_velocity[i_row].dot(velocity_dir[i_row]);

            for i_col in 0..nr_unknowns {
                let u_i_n = frozen_wake.variable_velocity_factors[[i_row, i_col]].dot(normal_dir[i_row]);

                // Lift at ctrl_point i_row due to induced angle of attack from line i_col
                equation_matrix[[i_row, i_col]] = -dcl_dalpha[i_row] * u_i_n / u_t;

                if i_row == i_col {
                    // Lift due to circulation
                    equation_matrix[[i_row, i_col]] += 2.0 / ( line_force_model.chord_lengths[i_row] * u_t);
                }
            }

            rhs[i_row] = -cl_linear[i_row];
        }

        // Solve for the circulation strength

        let mut circulation_strength = equation_matrix.solve_gaussian_elimination(&rhs).unwrap();

        frozen_wake.update_induced_velocities_at_control_points(&circulation_strength); 

        // TODO: move this somewhere else, to avoid code duplication
        match &self.velocity_corrections {
            VelocityCorrections::None => {
                for i in 0..nr_unknowns {
                    ctrl_point_velocity[i] = felt_ctrl_points_freestream[i] + frozen_wake.induced_velocities_at_control_points[i];
                }
            },
            VelocityCorrections::MaxInducedVelocityMagnitudeRatio(ratio) => {
                ctrl_point_velocity = VelocityCorrections::max_induced_velocity_magnitude_ratio(
                    *ratio, 
                    felt_ctrl_points_freestream, 
                    &frozen_wake.induced_velocities_at_control_points
                );
            },
            VelocityCorrections::FixedMagnitudeEqualToFreestream => {
                ctrl_point_velocity = VelocityCorrections::fixed_magnitude_equal_to_freestream(
                    felt_ctrl_points_freestream, 
                    &frozen_wake.induced_velocities_at_control_points
                );
            },
        }

        // Calculate corrections on the circulation due to viscous effects
        if !self.disable_viscous_corrections {
            angles_of_attack = line_force_model.angles_of_attack(
                &ctrl_point_velocity, 
                CoordinateSystem::Global
            );

            let cl_linear = line_force_model.lift_coefficients_linear(&angles_of_attack, &ctrl_point_velocity);

            let cl_full = line_force_model.lift_coefficients_pre_stall_with_stall_drop_off(
                &angles_of_attack,
                &ctrl_point_velocity,
            );

            for i in 0..circulation_strength.len() {
                circulation_strength[i] *= cl_full[i] / cl_linear[i]
            }

            match &self.final_circulation_correction_method {
                FinalCirculationCorrectionMethod::NoCorrection => {},
                FinalCirculationCorrectionMethod::IncludeViscousLiftInEffectiveCirculation => {
                    let cl_viscous = line_force_model.lift_coefficients_post_stall_with_stall_weight(
                        &angles_of_attack
                    );

                    let mut effective_circulation = circulation_strength.clone();

                    for i in 0..nr_unknowns {
                        effective_circulation[i] *= (cl_viscous[i] + cl_full[i]) / cl_full[i];
                    }

                    frozen_wake.update_induced_velocities_at_control_points(&effective_circulation);
                },
                FinalCirculationCorrectionMethod::FullCorrection => {
                    frozen_wake.update_induced_velocities_at_control_points(&circulation_strength);
                }
            }
            

            match &self.velocity_corrections {
                VelocityCorrections::None => {
                    for i in 0..nr_unknowns {
                        ctrl_point_velocity[i] = felt_ctrl_points_freestream[i] + frozen_wake.induced_velocities_at_control_points[i];
                    }
                },
                VelocityCorrections::MaxInducedVelocityMagnitudeRatio(ratio) => {
                    ctrl_point_velocity = VelocityCorrections::max_induced_velocity_magnitude_ratio(
                        *ratio, 
                        felt_ctrl_points_freestream, 
                        &frozen_wake.induced_velocities_at_control_points
                    );
                },
                VelocityCorrections::FixedMagnitudeEqualToFreestream => {
                    ctrl_point_velocity = VelocityCorrections::fixed_magnitude_equal_to_freestream(
                        felt_ctrl_points_freestream, 
                        &frozen_wake.induced_velocities_at_control_points
                    );
                },
            }
        }

        angles_of_attack = line_force_model.angles_of_attack(
            &ctrl_point_velocity, 
            CoordinateSystem::Global
        );

        let residual = line_force_model.average_residual_absolute(
            &circulation_strength, 
            &angles_of_attack,
            &ctrl_point_velocity
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
 