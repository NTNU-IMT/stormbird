// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};

use crate::math_utils::statistics;

use crate::vec3::Vec3;
use crate::line_force_model::prelude::*;

use crate::empirical_models::viscous_wakes::ViscousWakes;

use crate::lifting_line::wake_models::steady::{
    SteadyWake,
    SteadyWakeBuilder
};

use crate::io_structs::prelude::*;

use crate::section_models::SectionModel;

use crate::line_force_model::solver_utils::ConvergenceTest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadySolverSettings {
    #[serde(default="SteadySolverSettings::default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default="SteadySolverSettings::default_damping_factor")]
    pub damping_factor: f64,
    #[serde(default)]
    pub include_viscous_wake: bool,
    #[serde(default)]
    pub smoothing_length_ratio: Option<f64>,
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
            include_viscous_wake: Default::default(),
            smoothing_length_ratio: Default::default(),
            convergence_test: Default::default(),
            print_log: Default::default(),
        }
    }
}

/// Uses a line force model builder to construct multiple line force models with varying resolution. 
/// The algorithm starts by solving a coarse model and then gradually increases the resolution until 
/// a max value for the resolution is reached. The solution from the previous coarse model is used 
/// as initial conditions for a new fine model. 
/// 
/// The reason this solver is implemented is based on two oberservations:
/// 1) - coarse models seems to require fewer iterations than fine mdoels
/// 2) - coarse models seem to be more stable than fine models
/// 
/// The idea is inspired from multi grid solvers in CFD
pub fn solve_steady_multiresolution(
    line_force_model_builder: &LineForceModelBuilder, 
    velocity_input: &VelocityInput, 
    solver_settings: &SteadySolverSettings,
    wake_builder: &SteadyWakeBuilder
) -> SimulationResult {
    let end_correction = match line_force_model_builder.wing_builders[0].section_model {
        SectionModel::RotatingCylinder(_) => false,
        _ => true
    };

    let mut nr_sections: Vec<usize> = Vec::new();
    let mut current_nr_sections = line_force_model_builder.nr_sections;

    while current_nr_sections > 1 {
        nr_sections.push(current_nr_sections);

        if current_nr_sections % 2 == 0 {
            current_nr_sections /= 2;
        } else {
            current_nr_sections = (current_nr_sections + 1) / 2;
        }
    }

    nr_sections.reverse();

    let mut force_models: Vec<LineForceModel> = Vec::new();
    let mut results: Vec<SimulationResult> = Vec::new();

    for nr in nr_sections {
        force_models.push(
            line_force_model_builder.build_with_nr_sections(nr)
        );
    }

    for i in 0..force_models.len() {
        let initial_solution: Vec<f64> = if i==0 {
            vec![0.0; force_models[i].nr_span_lines()]
        } else {
            force_models[i-1].map_strength_gaussian(
                &results[i-1].circulation_strength, &force_models[i], (end_correction, end_correction), 0.5
            )
        };

        results.push(
            solve_steady(&force_models[i], velocity_input, solver_settings, wake_builder, &initial_solution)
        );
    }

    results.last().unwrap().clone()
}

pub fn solve_steady(
    line_force_model: &LineForceModel, 
    velocity_input: &VelocityInput, 
    solver_settings: &SteadySolverSettings,
    wake_builder: &SteadyWakeBuilder,
    initial_solution: &[f64],
) -> SimulationResult {
    let nr_ctrl_points = line_force_model.nr_span_lines();    

    let wake = wake_builder.build(
        line_force_model, 
        velocity_input, 
    );
    
    let mut circulation_strength: Vec<f64> = initial_solution.to_vec();

    let viscous_wakes: Option<ViscousWakes> = if solver_settings.include_viscous_wake {
        Some(ViscousWakes::from_line_force_model(line_force_model))
    } else {
        None
    };

    let mut convergence_test = solver_settings.convergence_test.build();

    let ctrl_points = line_force_model.ctrl_points();

    for iteration in 0..solver_settings.max_iterations {
        let velocity = calculate_velocity(
            line_force_model,
            &velocity_input,
            &wake, 
            viscous_wakes.as_ref(), 
            &circulation_strength
        );
        
        let new_estimated_strength = line_force_model.circulation_strength(&velocity);

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

    let velocity = calculate_velocity(
        line_force_model,
        &velocity_input,
        &wake, 
        viscous_wakes.as_ref(), 
        &circulation_strength
    );

    let sectional_forces = line_force_model.sectional_forces(&circulation_strength, &velocity);

    let integrated_forces = line_force_model.integrated_forces(&circulation_strength, &velocity);
    let integrated_moments = line_force_model.integrated_moments(&circulation_strength, &velocity);

    SimulationResult {
        ctrl_points,
        circulation_strength, 
        velocity,
        sectional_forces,
        integrated_forces,
        integrated_moments,
    }
}

/// Function that calculates the velocity at the control points of the wing.
/// 
/// Collected in a function as the same procedure is used multiple times in the solver.
fn calculate_velocity(
    line_force_model: &LineForceModel,
    force_input: &ForceInput,
    wake: &SteadyWake, 
    viscous_wakes: Option<&ViscousWakes>, 
    circulation_strength: &[f64]
) -> Vec<Vec3> {
    let ctrl_point_velocities: Vec<Vec3> = force_input.velocity.clone();

    let induced_velocities: Vec<Vec3> = wake.induced_velocities_at_control_points(circulation_strength);

    let mut velocity: Vec<Vec3> = ctrl_point_velocities.iter()
        .zip(induced_velocities.iter())
        .map(| (u_ctrl, u_i)| {*u_ctrl + *u_i})
        .collect();

    if let Some(viscous_wakes) = viscous_wakes.as_ref() {
        velocity = viscous_wakes.corrected_velocity(line_force_model, &velocity);
    }

    velocity
}