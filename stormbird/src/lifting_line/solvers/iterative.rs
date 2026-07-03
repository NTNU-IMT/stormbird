// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Settings for the solvers used in the lifting line models.

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::line_force_model::prelude::*;
use crate::common_utils::prelude::*;
use crate::lifting_line::wake::prelude::*;
use super::velocity_corrections::VelocityCorrections;

use super::linearized::Linearized;

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub enum VariableToUpdate {
    #[default]
    CirculationStrength,
    AngleOfAttack
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuasiSteadyIterativeBuilder {
    #[serde(default="QuasiSteadyIterativeBuilder::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="QuasiSteadyIterativeBuilder::default_damping_factor")]
    pub damping_factor: Float,
    #[serde(default="Iterative::default_residual_tolerance_absolute")]
    pub residual_tolerance_absolute: Float,
    #[serde(default="Iterative::default_strength_difference_tolerance")]
    pub strength_difference_tolerance: Float,
    #[serde(default)]
    pub velocity_corrections: VelocityCorrections,
    #[serde(default)]
    pub start_with_linearized_solution: bool,
    #[serde(default="QuasiSteadyIterativeBuilder::default_one")]
    pub correction_factor_initial_solution: Float,
    #[serde(default)]
    pub variable_to_update: VariableToUpdate
}

impl QuasiSteadyIterativeBuilder {
    pub fn default_max_iterations_per_time_step() -> usize {1000}
    pub fn default_damping_factor() -> Float {0.04}
    pub fn default_one() -> Float {1.0}

    pub fn build(&self) -> Iterative {
        Iterative {
            max_iterations_per_time_step: self.max_iterations_per_time_step,
            damping_factor: self.damping_factor,
            residual_tolerance_absolute: self.residual_tolerance_absolute,
            strength_difference_tolerance: self.strength_difference_tolerance,
            velocity_corrections: self.velocity_corrections.clone(),
            use_raw_circulation_during_iterations: false,
            start_with_linearized_solution: self.start_with_linearized_solution,
            correction_factor_initial_solution: self.correction_factor_initial_solution,
            variable_to_update: self.variable_to_update
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Iterative {
    #[serde(default="Iterative::default_max_iterations_per_time_step")]
    pub max_iterations_per_time_step: usize,
    #[serde(default="Iterative::default_damping_factor")]
    pub damping_factor: Float,
    #[serde(default="Iterative::default_residual_tolerance_absolute")]
    pub residual_tolerance_absolute: Float,
    #[serde(default="Iterative::default_strength_difference_tolerance")]
    pub strength_difference_tolerance: Float,
    #[serde(default)]
    pub velocity_corrections: VelocityCorrections,
    #[serde(default="Iterative::default_use_raw_circulation_during_iterations")]
    pub use_raw_circulation_during_iterations: bool,
    #[serde(default)]
    pub start_with_linearized_solution: bool,
    #[serde(default="Iterative::default_one")]
    pub correction_factor_initial_solution: Float,
    #[serde(default)]
    pub variable_to_update: VariableToUpdate
}

impl Iterative {
    pub fn default_use_raw_circulation_during_iterations() -> bool {false}
    pub fn default_max_iterations_per_time_step() -> usize {40}
    pub fn default_damping_factor() -> Float {0.05}
    pub fn default_residual_tolerance_absolute() -> Float {1e-4}
    pub fn default_strength_difference_tolerance() -> Float {1e-6}
    pub fn default_one() -> Float {1.0}

    pub fn solve(
        &self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector],
        frozen_wake: &mut FrozenWake,
        initial_solution: &[Float],
    ) -> SolverResult {
        let mut circulation_strength = self.initialize_circulation(
            line_force_model,
            felt_ctrl_points_freestream,
            frozen_wake,
            initial_solution
        );

        let mut ctrl_points_velocity = self.calculate_ctrl_point_velocity(
            line_force_model, 
            felt_ctrl_points_freestream, 
            frozen_wake, 
            &circulation_strength
        );
        
        let mut angles_of_attack = line_force_model.angles_of_attack(
            &ctrl_points_velocity,
            CoordinateSystem::Global
        );

        let mut iterations = 0;
        let mut converged = false;
        let mut residual = 0.0;
        while iterations < self.max_iterations_per_time_step && !converged {
            iterations += 1;

            ctrl_points_velocity = self.calculate_ctrl_point_velocity(
                line_force_model, 
                felt_ctrl_points_freestream, 
                frozen_wake, 
                &circulation_strength
            );

            match self.variable_to_update {
                VariableToUpdate::CirculationStrength => {
                    (converged, residual) = self.update_circulation_strength(
                        line_force_model, 
                        &ctrl_points_velocity, 
                        &mut circulation_strength,
                        &mut angles_of_attack,
                    );
                },
                VariableToUpdate::AngleOfAttack => {
                    (converged, residual) = self.update_angle_of_attack(
                        line_force_model, 
                        &ctrl_points_velocity, 
                        &mut circulation_strength,
                        &mut angles_of_attack,
                    );
                }
            }
            
        }

        SolverResult {
            input_ctrl_points_velocity: felt_ctrl_points_freestream.to_vec(),
            circulation_strength,
            output_ctrl_points_velocity: ctrl_points_velocity,
            iterations,
            residual
        }
    }

    fn update_circulation_strength(
        &self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        circulation_strength: &mut [Float],
        angles_of_attack: &mut [Float]
    ) -> (bool, Float) {
        let nr_ctrl_points = line_force_model.nr_span_lines();

        let new_angles_of_attack = line_force_model.angles_of_attack(
            &ctrl_points_velocity,
            CoordinateSystem::Global
        );

        for i in 0..angles_of_attack.len() {
            angles_of_attack[i] = new_angles_of_attack[i];
        }

        let new_estimated_strength = if self.use_raw_circulation_during_iterations {
            line_force_model.circulation_strength_raw(
                &angles_of_attack,
                &ctrl_points_velocity
            )
        } else {
            line_force_model.circulation_strength(
                &angles_of_attack,
                &ctrl_points_velocity
            )
        };

        let mut max_strength_difference = 0.0;

        for i in 0..nr_ctrl_points {
            let strength_difference = new_estimated_strength[i] - circulation_strength[i];

            let absolute_strength_difference = strength_difference.abs();

            if absolute_strength_difference > max_strength_difference {
                max_strength_difference = absolute_strength_difference;
            }

            circulation_strength[i] += self.damping_factor * strength_difference;
        }

        let residual = line_force_model.average_residual_absolute(
            &circulation_strength,
            &angles_of_attack,
            &ctrl_points_velocity
        );

        let mut converged = false;

        if residual < self.residual_tolerance_absolute {
            converged = true;
        }

        if max_strength_difference < self.strength_difference_tolerance {
            converged = true;
        }

        (converged, residual)
    }

    fn update_angle_of_attack(
        &self,
        line_force_model: &LineForceModel,
        ctrl_points_velocity: &[SpatialVector],
        circulation_strength: &mut [Float],
        angles_of_attack: &mut [Float],
    ) -> (bool, Float) {
        let nr_ctrl_points = line_force_model.nr_span_lines();

        let new_angles_of_attack = line_force_model.angles_of_attack(
            &ctrl_points_velocity,
            CoordinateSystem::Global
        );

        for i in 0..nr_ctrl_points {
            let angle_difference = new_angles_of_attack[i] - angles_of_attack[i];
            angles_of_attack[i] += self.damping_factor * angle_difference;
        }

        let new_estimated_strength = if self.use_raw_circulation_during_iterations {
            line_force_model.circulation_strength_raw(
                &angles_of_attack,
                &ctrl_points_velocity
            )
        } else {
            line_force_model.circulation_strength(
                &angles_of_attack,
                &ctrl_points_velocity
            )
        };

        let mut max_strength_difference = 0.0;

        for i in 0..nr_ctrl_points {
            let strength_difference = new_estimated_strength[i] - circulation_strength[i];

            let absolute_strength_difference = strength_difference.abs();

            if absolute_strength_difference > max_strength_difference {
                max_strength_difference = absolute_strength_difference;
            }

            circulation_strength[i] = new_estimated_strength[i];
        }

        let residual = line_force_model.average_residual_absolute(
            &circulation_strength,
            &angles_of_attack,
            &ctrl_points_velocity
        );

        let mut converged = false;

        if residual < self.residual_tolerance_absolute {
            converged = false;
        }

        if max_strength_difference < self.strength_difference_tolerance {
            converged = false;
        }

        (converged, residual)
    }

    /// Create an initial estimate of the circulation strength at the first iteration
    fn initialize_circulation(
        &self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector],
        frozen_wake: &mut FrozenWake,
        initial_solution: &[Float],
    ) -> Vec<Float> {
        let mut circulation_strength: Vec<Float> = initial_solution.to_vec();
        
        if (self.correction_factor_initial_solution - 1.0).abs() > Float::MIN_POSITIVE {
            for i in 0..circulation_strength.len() {
                circulation_strength[i] *= self.correction_factor_initial_solution;
            }
        }

        if self.start_with_linearized_solution {
            let linearized_solver = Linearized::default();

            let linearized_result = linearized_solver.solve(
                line_force_model,
                felt_ctrl_points_freestream,
                frozen_wake
            );

            circulation_strength = linearized_result.circulation_strength;
        }

        circulation_strength
    }

    /// Calculates the ctrl point velocity from the given circulation strength
    fn calculate_ctrl_point_velocity(
        &self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector],
        frozen_wake: &mut FrozenWake,
        circulation_strength: &[Float],
    ) -> Vec<SpatialVector> {
        let nr_ctrl_points = line_force_model.nr_span_lines();
        
        let mut ctrl_points_velocity = vec![SpatialVector::default(); nr_ctrl_points];

        frozen_wake.update_induced_velocities_at_control_points(
            &circulation_strength
        );

        let corrected_velocity = self.velocity_corrections.corrected_velocity(
            felt_ctrl_points_freestream,
            &frozen_wake.induced_velocities_at_control_points
        );

        if let Some(velocity) = corrected_velocity {
            ctrl_points_velocity = velocity
        } else {
            for i in 0..nr_ctrl_points {
                ctrl_points_velocity[i] = felt_ctrl_points_freestream[i] +
                    frozen_wake.induced_velocities_at_control_points[i];
            }
        }

        ctrl_points_velocity = line_force_model.remove_span_velocity(
            &ctrl_points_velocity,
            CoordinateSystem::Global
        );

        ctrl_points_velocity
    }

    
}

impl Default for Iterative {
    fn default() -> Self {
        Iterative {
            max_iterations_per_time_step: Iterative::default_max_iterations_per_time_step(),
            damping_factor: Iterative::default_damping_factor(),
            residual_tolerance_absolute: Iterative::default_residual_tolerance_absolute(),
            strength_difference_tolerance: Iterative::default_strength_difference_tolerance(),
            velocity_corrections: VelocityCorrections::default(),
            use_raw_circulation_during_iterations: Iterative::default_use_raw_circulation_during_iterations(),
            start_with_linearized_solution: false,
            correction_factor_initial_solution: Iterative::default_one(),
            variable_to_update: VariableToUpdate::default()
        }
    }
}

impl Default for QuasiSteadyIterativeBuilder {
    fn default() -> Self {
        QuasiSteadyIterativeBuilder {
            max_iterations_per_time_step: QuasiSteadyIterativeBuilder::default_max_iterations_per_time_step(),
            damping_factor: QuasiSteadyIterativeBuilder::default_damping_factor(),
            residual_tolerance_absolute: Iterative::default_residual_tolerance_absolute(),
            strength_difference_tolerance: Iterative::default_strength_difference_tolerance(),
            velocity_corrections: VelocityCorrections::default(),
            start_with_linearized_solution: false,
            correction_factor_initial_solution: QuasiSteadyIterativeBuilder::default_one(),
            variable_to_update: VariableToUpdate::default()
        }
    }
}
