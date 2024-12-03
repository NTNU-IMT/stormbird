// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Settings for the solvers used in the lifting line models.

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;

use crate::line_force_model::prelude::*;
use crate::io_structs::prelude::*;
use crate::lifting_line::wake::prelude::*;
use super::velocity_corrections::VelocityCorrections;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadySimpleIterativeBuilder {
    #[serde(default="SteadySimpleIterativeBuilder::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="SteadySimpleIterativeBuilder::default_damping_factor")]
    pub damping_factor: f64,
    #[serde(default="SimpleIterative::default_residual_tolerance_absolute")]
    pub residual_tolerance_absolute: f64,
    #[serde(default="SimpleIterative::default_strength_difference_tolerance")]
    pub strength_difference_tolerance: f64,
    #[serde(default)]
    pub velocity_corrections: VelocityCorrections,
}

impl SteadySimpleIterativeBuilder {
    pub fn default_max_iterations_per_time_step() -> usize {1000}
    pub fn default_damping_factor() -> f64 {0.04}

    pub fn build(&self) -> SimpleIterative {
        SimpleIterative {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor: self.damping_factor,
            residual_tolerance_absolute: self.residual_tolerance_absolute,
            strength_difference_tolerance: self.strength_difference_tolerance,
            velocity_corrections: self.velocity_corrections.clone(),
            use_raw_circulation_during_iterations: false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimpleIterative {
    #[serde(default="SimpleIterative::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="SimpleIterative::default_damping_factor")]
    pub damping_factor: f64,
    #[serde(default="SimpleIterative::default_residual_tolerance_absolute")]
    pub residual_tolerance_absolute: f64,
    #[serde(default="SimpleIterative::default_strength_difference_tolerance")]
    pub strength_difference_tolerance: f64,
    #[serde(default)]
    pub velocity_corrections: VelocityCorrections,
    #[serde(default="SimpleIterative::default_use_raw_circulation_during_iterations")]
    pub use_raw_circulation_during_iterations: bool,
}

impl SimpleIterative {
    pub fn default_use_raw_circulation_during_iterations() -> bool {false}
    pub fn default_max_iterations_per_time_step() -> usize {10}
    pub fn default_damping_factor() -> f64 {0.04}
    pub fn default_residual_tolerance_absolute() -> f64 {1e-4}
    pub fn default_strength_difference_tolerance() -> f64 {1e-6}

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
            &ctrl_point_velocity,
            CoordinateSystem::Global
        );
        
        let mut iterations = 0;
        let mut converged = false;
        while iterations < self.max_iterations_per_time_step && !converged {
            iterations += 1;
    
            let induced_velocities = frozen_wake.induced_velocities_at_control_points(&circulation_strength);

            match &self.velocity_corrections {
                VelocityCorrections::None => {
                    for i in 0..ctrl_points.len() {
                        ctrl_point_velocity[i] = felt_ctrl_points_freestream[i] + induced_velocities[i];
                    }
                },
                VelocityCorrections::MaxInducedVelocityMagnitudeRatio(ratio) => {
                    ctrl_point_velocity = VelocityCorrections::max_induced_velocity_magnitude_ratio(
                        *ratio, 
                        felt_ctrl_points_freestream, 
                        &induced_velocities
                    );
                },
                VelocityCorrections::FixedMagnitudeEqualToFreestream => {
                    ctrl_point_velocity = VelocityCorrections::fixed_magnitude_equal_to_freestream(
                        felt_ctrl_points_freestream, 
                        &induced_velocities
                    );
                },
            }

            ctrl_point_velocity = line_force_model.remove_span_velocity(&ctrl_point_velocity, CoordinateSystem::Global);
    
            let new_estimated_strength = if self.use_raw_circulation_during_iterations {
                line_force_model.circulation_strength_raw(&ctrl_point_velocity, CoordinateSystem::Global)
            } else {
                line_force_model.circulation_strength(&ctrl_point_velocity, CoordinateSystem::Global)
            };
    
            residual = line_force_model.average_residual_absolute(
                &circulation_strength, 
                &ctrl_point_velocity,
                CoordinateSystem::Global
            );
    
            if residual < self.residual_tolerance_absolute {
                converged = true;
            }
    
            let damping_factor = self.damping_factor;

            let mut max_strength_difference = 0.0;
    
            for i in 0..ctrl_points.len() {
                let strength_difference = new_estimated_strength[i] - circulation_strength[i];

                let absolute_strength_difference = strength_difference.abs();

                if absolute_strength_difference > max_strength_difference {
                    max_strength_difference = absolute_strength_difference;
                }

                circulation_strength[i] += damping_factor * strength_difference;
            }

            if max_strength_difference < self.strength_difference_tolerance {
                converged = true;
            }
        }

        circulation_strength = line_force_model.circulation_strength(&ctrl_point_velocity, CoordinateSystem::Global);
    
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
            residual_tolerance_absolute: SimpleIterative::default_residual_tolerance_absolute(),
            strength_difference_tolerance: SimpleIterative::default_strength_difference_tolerance(),
            velocity_corrections: VelocityCorrections::default(),
            use_raw_circulation_during_iterations: SimpleIterative::default_use_raw_circulation_during_iterations(),
        }
    }
}

impl Default for SteadySimpleIterativeBuilder {
    fn default() -> Self {
        SteadySimpleIterativeBuilder {
            max_iterations_per_time_step: SteadySimpleIterativeBuilder::default_max_iterations_per_time_step(),
            damping_factor: SteadySimpleIterativeBuilder::default_damping_factor(),
            residual_tolerance_absolute: SimpleIterative::default_residual_tolerance_absolute(),
            strength_difference_tolerance: SimpleIterative::default_strength_difference_tolerance(),
            velocity_corrections: VelocityCorrections::default(),
        }
    }
}