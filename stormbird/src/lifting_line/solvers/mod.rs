// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Lifting line solvers
//! 
use crate::math_utils::statistics;
use crate::vec3::Vec3;
use crate::line_force_model::prelude::*;
use crate::io_structs::prelude::*;
use crate::lifting_line::wake_models::prelude::*;

pub mod settings;

use settings::SolverSettings;

pub fn solve_time_step(
    line_force_model: &LineForceModel,
    felt_ctrl_points_freestream: &[Vec3],
    solver_settings: &SolverSettings,
    wake_model: &mut WakeModel,
    initial_solution: &[f64],
) -> SolverResult {
    let ctrl_points = line_force_model.ctrl_points();

    let mut convergence_test = solver_settings.convergence_test.build();

    let fixed_velocities = match wake_model {
        WakeModel::Steady((_, _)) => {
            felt_ctrl_points_freestream.to_vec()
        },
        WakeModel::Unsteady(wake) => {
            let u_i_free_wake = wake.induced_velocities_from_free_wake(&ctrl_points, false);

            (0..ctrl_points.len()).map(|i| {
                    felt_ctrl_points_freestream[i] + u_i_free_wake[i]
                }
            ).collect()
        }
    };

    let mut circulation_strength: Vec<f64> = initial_solution.to_vec();
    let mut velocity = vec![Vec3::default(); ctrl_points.len()];

    for iteration in 0..solver_settings.max_iterations_per_time_step {
        let velocity_update = match wake_model {
            WakeModel::Steady((_, wake)) => {
                wake.induced_velocities_at_control_points(&circulation_strength)
            },
            WakeModel::Unsteady(wake) => {
                wake.induced_velocities_from_first_panels(&ctrl_points, false)
            }
        };

        for i in 0..ctrl_points.len() {
            velocity[i] = fixed_velocities[i] + velocity_update[i];
        }

        let mut new_estimated_strength = line_force_model.circulation_strength(&velocity);

        if solver_settings.circulation_viscosity != 0.0 {
            let circulation_strength_second_derivative = line_force_model.circulation_strength_second_derivative(
                &new_estimated_strength
            );

            for i in 0..ctrl_points.len() {
                let viscosity_term = solver_settings.circulation_viscosity * circulation_strength_second_derivative[i];
                
                new_estimated_strength[i] += viscosity_term;
            }
        }

        if let Some(smoothing_length) = solver_settings.gaussian_smoothing_length {
            new_estimated_strength = line_force_model.gaussian_smoothed_strength(
                &new_estimated_strength, smoothing_length
            );
        }

        let strength_difference: Vec<f64> = (0..ctrl_points.len()).map(|i| {
            (new_estimated_strength[i] - circulation_strength[i]).abs()
        }).collect();

        let max_strength_difference = statistics::max(&strength_difference);

        if convergence_test.test(max_strength_difference) {
            if solver_settings.print_log {
                println!(
                    "Converged after {} iterations with {} sections", iteration+1, ctrl_points.len()
                );
            }
            
            break;
        }

        for i in 0..ctrl_points.len() {
            let strength_difference = new_estimated_strength[i] - circulation_strength[i];
            circulation_strength[i] += solver_settings.damping_factor * strength_difference;
        }

        if let WakeModel::Unsteady(wake) = wake_model {
            wake.update_wing_strength(&circulation_strength);
        }
    }

    SolverResult {
        circulation_strength,
        ctrl_point_velocity: velocity,
    }
}