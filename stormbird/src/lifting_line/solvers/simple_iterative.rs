// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Settings for the solvers used in the lifting line models.

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;

use crate::line_force_model::prelude::*;
use crate::io_structs::prelude::*;
use crate::lifting_line::wake::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimpleIterative {
    #[serde(default="SimpleIterative::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="SimpleIterative::default_damping_factor")]
    pub damping_factor: f64,
    #[serde(default="SimpleIterative::default_tolerance_absolute")]
    pub tolerance_absolute: f64,
}

impl SimpleIterative {
    pub fn default_max_iterations_per_time_step() -> usize {1000}
    pub fn default_damping_factor() -> f64 {0.02}
    pub fn default_tolerance_absolute() -> f64 {1e-4}

    pub fn do_step(
        &self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector<3>],
        frozen_wake: &FrozenWake,
        initial_solution: &[f64],
    ) -> SolverResult {
        let ctrl_points = line_force_model.ctrl_points();
    
        let mut circulation_strength: Vec<f64> = initial_solution.to_vec();
        let mut ctrl_point_velocity = vec![SpatialVector::<3>::default(); ctrl_points.len()];
        let mut residual = line_force_model.average_residual_absolute(
            &circulation_strength, 
            &ctrl_point_velocity
        );
        
        let mut iterations = 0;
        let mut converged = false;
        while iterations < self.max_iterations_per_time_step && !converged {
            iterations += 1;
    
            let induced_velocities = frozen_wake.induced_velocities_at_control_points(&circulation_strength);
    
            for i in 0..ctrl_points.len() {
                ctrl_point_velocity[i] = felt_ctrl_points_freestream[i] + induced_velocities[i];
            }
    
            ctrl_point_velocity = line_force_model.remove_span_velocity(&ctrl_point_velocity);
    
    
            let new_estimated_strength = line_force_model.circulation_strength(&ctrl_point_velocity);
    
            residual = line_force_model.average_residual_absolute(
                &circulation_strength, 
                &ctrl_point_velocity
            );
    
            if residual < self.tolerance_absolute {
                converged = true;
            }
    
            let damping_factor = self.damping_factor;
    
            for i in 0..ctrl_points.len() {
                let strength_difference = new_estimated_strength[i] - circulation_strength[i];
                circulation_strength[i] += damping_factor * strength_difference;
            }
        }
    
        SolverResult {
            circulation_strength,
            ctrl_point_velocity,
            iterations,
            residual
        }
    }
}

impl Default for SimpleIterative {
    fn default() -> Self {
        SimpleIterative {
            max_iterations_per_time_step: SimpleIterative::default_max_iterations_per_time_step(),
            damping_factor: SimpleIterative::default_damping_factor(),
            tolerance_absolute: SimpleIterative::default_tolerance_absolute(),
        }
    }
}