// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use crate::math_utils::statistics;

use crate::vec3::Vec3;
use crate::line_force_model::prelude::*;

use crate::lifting_line::wake_models::steady::{
    SteadyWakeBuilder
};

use crate::io_structs::prelude::*;

use crate::line_force_model::solver_utils::ConvergenceTest;

use super::SolverResult;
use super::calculate_felt_ctrl_points_freestream;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadySolverSettings {
    #[serde(default="SteadySolverSettings::default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default="SteadySolverSettings::default_damping_factor")]
    pub damping_factor: f64,
    #[serde(default)]
    pub circulation_viscosity: f64,
    #[serde(default)]
    pub convergence_test: ConvergenceTest,
    #[serde(default)]
    pub print_log: bool,
}

impl SteadySolverSettings {
    pub fn default_max_iterations() -> usize {1000}
    pub fn default_damping_factor() -> f64 {0.05}
}

impl Default for SteadySolverSettings {
    fn default() -> Self {
        SteadySolverSettings {
            max_iterations: Self::default_max_iterations(),
            damping_factor: Self::default_damping_factor(),
            circulation_viscosity: Default::default(),
            convergence_test: Default::default(),
            print_log: Default::default(),
        }
    }
}

pub fn solve_steady(
    time_step: f64,
    line_force_model: &LineForceModel, 
    ctrl_points_freestream: &[Vec3],
    derivatives: Option<&Derivatives>,
    solver_settings: &SteadySolverSettings,
    wake_builder: &SteadyWakeBuilder,
    initial_solution: &[f64],
) -> SolverResult {
    let nr_ctrl_points = line_force_model.nr_span_lines();    

    let wake = wake_builder.build(
        line_force_model, 
        ctrl_points_freestream, 
    );
    
    let mut circulation_strength: Vec<f64> = initial_solution.to_vec();
    let mut velocity = vec![Vec3::default(); nr_ctrl_points];

    let mut convergence_test = solver_settings.convergence_test.build();

    let felt_ctrl_points_freestream = calculate_felt_ctrl_points_freestream(
        ctrl_points_freestream,
        time_step,
        derivatives,
        line_force_model,
    );

    for iteration in 0..solver_settings.max_iterations {
        let induced_velocities = wake.induced_velocities_at_control_points(&circulation_strength);

        for i in 0..nr_ctrl_points {
            velocity[i] = felt_ctrl_points_freestream[i] + induced_velocities[i];
        }
        
        let mut new_estimated_strength = line_force_model.circulation_strength(&velocity);

        if solver_settings.circulation_viscosity != 0.0 {
            let circulation_strength_second_derivative = line_force_model.circulation_strength_second_derivative(
                &new_estimated_strength
            );

            for i in 0..nr_ctrl_points {
                new_estimated_strength[i] += solver_settings.circulation_viscosity * circulation_strength_second_derivative[i];
            }
        }

        let strength_difference: Vec<f64> = new_estimated_strength
            .iter()
            .zip(&circulation_strength)
            .map(|(new, old)| (new - old).abs())
            .collect();

        let max_strength_difference = statistics::max(&strength_difference);

        if convergence_test.test(max_strength_difference) {
            if solver_settings.print_log {
                println!(
                    "Converged after {} iterations with {} sections", iteration+1, nr_ctrl_points
                );
            }
           
            break;
        }

        for i in 0..nr_ctrl_points {
            let strength_difference = new_estimated_strength[i] - circulation_strength[i];
            circulation_strength[i] += solver_settings.damping_factor * strength_difference;
        }

    }

    SolverResult {
        circulation_strength,
        ctrl_point_velocity: velocity,
    }
}