// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Unsteady lifting line solvers

use serde::{Serialize, Deserialize};

use crate::vec3::Vec3;
use crate::line_force_model::prelude::*;
use crate::io_structs::prelude::*;
use crate::lifting_line::wake_models::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsteadySolverSettings {
    #[serde(default="UnsteadySolverSettings::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="UnsteadySolverSettings::default_damping_factor")]
    pub damping_factor: f64
}

impl UnsteadySolverSettings {
    pub fn default_max_iterations_per_time_step() -> usize {20}
    pub fn default_damping_factor() -> f64 {0.1}
}

impl Default for UnsteadySolverSettings {
    fn default() -> Self {
        UnsteadySolverSettings {
            max_iterations_per_time_step: Self::default_max_iterations_per_time_step(),
            damping_factor: Self::default_damping_factor(),
        }
    }
}

/// Function to do one time step with an unsteady lifting line solver
pub fn solve_one_time_step(
    time_step: f64,
    line_force_model: &LineForceModel,
    freestream: &Freestream,
    motion: &Motion,
    wake: &mut UnsteadyWake,
    solver_settings: &UnsteadySolverSettings,
    previous_circulation_strength: &[f64]
) -> SimulationResult
{   
    wake.synchronize_wing_geometry(&line_force_model);
    
    let ctrl_points = line_force_model.ctrl_points();

    // Velocity components that are fixed for the entire time step    
    let u_inf_ctrl_point: Vec<Vec3> = freestream.velocity_at_locations(&ctrl_points);

    let u_i_free_wake: Vec<Vec3>    = wake.induced_velocities_from_free_wake(&ctrl_points, false);

    let mut fixed_velocities: Vec<Vec3> = Vec::with_capacity(ctrl_points.len());

    for i in 0..ctrl_points.len() {
        fixed_velocities.push(
            u_inf_ctrl_point[i] - motion.velocity[i] + u_i_free_wake[i]
        );
    }

    // Iterate to solver for the strength at the first panel
    let mut circulation_strength = previous_circulation_strength.to_vec();

    for _ in 0..solver_settings.max_iterations_per_time_step {
        let update_to_velocity = wake.induced_velocities_from_first_panels(&ctrl_points, false);
        
        let velocity: Vec<Vec3> = fixed_velocities.iter()
            .zip(update_to_velocity.iter())
            .map(|(a, b)| *a + *b).collect();

        let new_circulation_strength = line_force_model.circulation_strength(&velocity);

        for i in 0..circulation_strength.len() {
            let change = new_circulation_strength[i] - circulation_strength[i];
            
            circulation_strength[i] += solver_settings.damping_factor * change;
        }

        wake.update_wing_strength(&circulation_strength);
    }

    // Calculate the final velocity based on the final circulation strength
    let update_to_velocity = wake.induced_velocities_from_first_panels(&ctrl_points, false);
        
    let velocity: Vec<Vec3> = fixed_velocities.iter()
        .zip(update_to_velocity.iter())
        .map(|(a, b)| *a + *b).collect();

    // Do post processing
    let force_input = SectionalForcesInput {
        circulation_strength,
        felt_velocity: velocity,
        acceleration: motion.acceleration.clone(),
        chord_rotation_velocity: motion.chord_rotation_velocity.clone(),
    };

    let sectional_forces   = line_force_model.sectional_forces(&force_input);

    let integrated_forces = sectional_forces.integrate_forces(&line_force_model);
    let integrated_moments = sectional_forces.integrate_moments(&line_force_model);

    wake.update_after_completed_time_step(&force_input.circulation_strength, time_step, line_force_model, freestream);

    SimulationResult {
        ctrl_points,
        circulation_strength: force_input.circulation_strength, 
        velocity: force_input.felt_velocity,
        sectional_forces,
        integrated_forces,
        integrated_moments,
    }
}