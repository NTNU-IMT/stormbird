// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality to compute derivatives of different quantities relevant or the output from the 
//! line force model.

use stormath::spatial_vector::SpatialVector;
use stormath::finite_difference;



#[derive(Debug, Clone)]
/// Structure used to calculate the derivatives of flow quantities in a line force model
pub struct FlowDerivatives {
    pub velocity_history: [Vec<SpatialVector<3>>; 2],
    update_count: usize,
}

impl FlowDerivatives {
    /// Create a new FlowDerivativesCalculator
    pub fn new(initial_velocity: &[SpatialVector<3>]) -> Self {
        Self {
            velocity_history: [initial_velocity.to_vec(), initial_velocity.to_vec()],
            update_count: 0,
        }
    }

    /// Calculates the *flow acceleration* based on the stored history
    pub fn acceleration(&self, current_velocity: &[SpatialVector<3>], time_step: f64) -> Vec<SpatialVector<3>> {
        if self.update_count < 2 {
            return vec![SpatialVector::<3>::default(); current_velocity.len()];
        }

        let mut acceleration = Vec::with_capacity(current_velocity.len());

        for i in 0..current_velocity.len() {
            let data = [
                self.velocity_history[0][i],
                self.velocity_history[1][i],
                current_velocity[i],
            ];

            acceleration.push(
                finite_difference::first_derivative_second_order_backward(
                    &data,
                    time_step,
                )
            );
        }

        acceleration
    }


    pub fn update(&mut self, current_velocity: &[SpatialVector<3>]) {
        self.velocity_history[0] = self.velocity_history[1].clone();
        self.velocity_history[1] = current_velocity.to_vec();

        self.update_count += 1;
    }
}

/*impl LineForceModel {
    pub fn initialize_flow_derivatives(&mut self, ctrl_points_freestream: &[SpatialVector<3>]) {
        let initial_angles = self.angles_of_attack(
            ctrl_points_freestream, 
            CoordinateSystem::Global
        );

        self.flow_derivatives = Some(
            FlowDerivatives::new(
                ctrl_points_freestream,
                &initial_angles
            )
        )
    }

    pub fn update_flow_derivatives(&mut self, result: &SimulationResult) {        
        if let Some(derivatives) = self.flow_derivatives.as_mut() {
            derivatives.update(
                &result.force_input.velocity, 
                &result.force_input.angles_of_attack
            );
        }
    }
}*/