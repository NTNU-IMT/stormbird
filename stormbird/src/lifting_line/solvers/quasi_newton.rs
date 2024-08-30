
use core::f64;

use ndarray::prelude::*;
use ndarray_linalg::Solve;

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;
use crate::line_force_model::prelude::*;
use crate::io_structs::prelude::*;
use crate::lifting_line::wake::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuasiNewtonBuilder {
    pub damping_factor: f64,
    pub max_iterations: usize,
    pub tolerance_absolute: f64,
}

impl QuasiNewtonBuilder {
    pub fn build(&self, size: usize) -> QuasiNewton {
        let jacobian: Array2<f64> = Array2::eye(size);

        QuasiNewton {
            damping_factor: self.damping_factor,
            max_iterations: self.max_iterations,
            tolerance_absolute: self.tolerance_absolute,
            jacobian,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuasiNewton {
    pub damping_factor: f64,
    pub max_iterations: usize,
    pub tolerance_absolute: f64,
    /// The Jacobian matrix represents how the function value changes with respect to the all the 
    /// circulation strengths. The first row evaluates how the residual function changes with 
    /// changing values fro the circulation strengths. 
    /// 
    /// That is, first row contains df[0] / dx[i]
    pub jacobian: Array2<f64>,
}

impl QuasiNewton {
    pub fn initialize_jacobian(&mut self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector<3>],
        frozen_wake: &FrozenWake
    ) {
        let nr_span_lines = line_force_model.nr_span_lines();

        self.jacobian = Array2::zeros((nr_span_lines, nr_span_lines));

        let dx = 0.01;
        
        for x_index in 0..line_force_model.nr_span_lines() {
            let mut test_circulation: Vec<f64> = vec![0.0; nr_span_lines];

            test_circulation[x_index] = dx;

            let result_function = self.evaluate_function(
                line_force_model,
                &test_circulation,
                felt_ctrl_points_freestream,
                frozen_wake,
            ).0;
            
            for y_index in 0..nr_span_lines {
                self.jacobian[[y_index, x_index]] = (result_function[y_index]) / dx;
            }
        }

        
    }

    fn evaluate_function(
        &self,
        line_force_model: &LineForceModel,
        previous_circulation_strength: &[f64],
        felt_ctrl_points_freestream: &[SpatialVector<3>],
        frozen_wake: &FrozenWake,
    ) -> (Array1<f64>, Vec<SpatialVector<3>>) {
        let induced_velocities = 
            frozen_wake.induced_velocities_at_control_points(&previous_circulation_strength);

        let mut ctrl_point_velocity: Vec<SpatialVector<3>> = induced_velocities.iter()
            .zip(felt_ctrl_points_freestream.iter())
                .map(
                    |(u_i, u_inf)| {
                        *u_i + *u_inf
                    }
                ).collect();

        ctrl_point_velocity = line_force_model.remove_span_velocity(&ctrl_point_velocity);

        let new_estimated_strength = line_force_model.circulation_strength(&ctrl_point_velocity);

        let func_value = Array1::from_vec(
            line_force_model.residual(
                &new_estimated_strength, 
                &ctrl_point_velocity
            )
        );

        (func_value, ctrl_point_velocity)
    }

    pub fn do_step(
        &mut self,
        line_force_model: &LineForceModel,
        felt_ctrl_points_freestream: &[SpatialVector<3>],
        frozen_wake: &FrozenWake,
        initial_solution: &[f64],
    ) -> SolverResult {
        self.initialize_jacobian(line_force_model, felt_ctrl_points_freestream, frozen_wake);

        dbg!(&self.jacobian);

        let mut circulation_strength: Vec<f64> = initial_solution.to_vec();

        let (mut f, mut ctrl_point_velocity) = 
            self.evaluate_function(
                line_force_model,
                &circulation_strength,
                felt_ctrl_points_freestream,
                &frozen_wake,
            );

        let mut f_new: Array1<f64>;

        let mut residual = f64::INFINITY;
        
        let mut iterations = 0;
        let mut converged = false;
        while iterations < self.max_iterations && !converged {
            iterations += 1;

            let change_in_circulation = self.jacobian.solve(&-&f).unwrap();

            let new_circulation: Vec<f64> = circulation_strength.iter()
                .zip(change_in_circulation.iter())
                .map(
                    |(old, change)| {
                        old + change
                    }
                ).collect();

            (f_new, ctrl_point_velocity) = 
                self.evaluate_function(
                    line_force_model,
                    &new_circulation,
                    felt_ctrl_points_freestream,
                    &frozen_wake,
                );

            let delta_f = &f_new - &f;

            let delta_x_norm_squared: f64 = change_in_circulation.iter()
                .map(|x| x.powi(2))
                .sum::<f64>();

            if delta_x_norm_squared.abs() < 1e-10 {
                println!("Delta x norm squared is too small at iteration {}", iterations);
                break;
            }

            let delta_jacobian = (delta_f - self.jacobian.dot(&change_in_circulation))
                .dot(&change_in_circulation.t()) / delta_x_norm_squared;

            self.jacobian += delta_jacobian;
            
            f = f_new;
            circulation_strength = new_circulation;

            residual = line_force_model.average_residual_absolute(
                &circulation_strength, 
                &ctrl_point_velocity
            );

            if residual < self.tolerance_absolute {
                converged = true;
            }
            
        }

        SolverResult {
            circulation_strength,
            ctrl_point_velocity,
            iterations,
            residual,
        }
    }
}