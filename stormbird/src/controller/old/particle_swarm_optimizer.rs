// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::optimize::particle_swarm::{
    SwarmState,
    SwarmResult,
    ParticleSwarm
};

use stormath::type_aliases::Float;
use stormath::spatial_vector::SpatialVector;

use stormath::statistics::time_averaged_mean;

use crate::common_utils::results::simulation::SimulationResult;

use serde::{Serialize, Deserialize};

use super::ControllerOutput;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// This structure is used to control an optimization process that can run in dynamic simulations. 
pub struct ParticleSwarmOptimizer {
    pub initialization_time: Float,
    pub time_between_evaluations: Float,
    pub averaging_time: Float,
    pub thrust_direction: SpatialVector,
    pub optimizer: ParticleSwarm,
    #[serde(default)]
    pub current_particle: usize,
    #[serde(default)]
    pub current_generation: usize,
    #[serde(default)]
    pub state_history: Vec<SwarmState>,
    #[serde(default)]
    pub result_history: Vec<SwarmResult>,
}

impl ParticleSwarmOptimizer {
    pub fn objective_function(&self, time: &[Float], result: &[SimulationResult]) -> Float {
        let mut relevant_thrust_values: Vec<Float> = Vec::new();
        let mut relevant_times: Vec<Float> = Vec::new();

        let time_length = time.len();

        let time_window_start = time[time_length - 1] - self.averaging_time;

        for i in 0..time.len() {
            if time[i] > time_window_start {
                let force = result[i].integrated_forces_sum();

                let thrust = force.dot(self.thrust_direction);

                relevant_thrust_values.push(thrust);
                relevant_times.push(time[i]);
            }
        }
        
        -time_averaged_mean(&relevant_times, &relevant_thrust_values)
    }

    /// Returns the index of the current test in the optimization process.
    pub fn test_index(&self) -> usize {
        self.current_generation * self.optimizer.nr_particles + self.current_particle
    }

    /// Updates the particle and generation index for the next test.
    pub fn increase_test_index(&mut self) {
        self.current_particle += 1;

        if self.current_particle >= self.optimizer.nr_particles {
            self.current_particle = 0;
            self.current_generation += 1;
        }
    }

    /// Check if a new particle generation is required.
    pub fn new_generation(&self) -> bool {
        if self.current_particle == 0 {
            true
        } else {
            false
        }
    }

    /// Checks the time to see if a new test is required.
    pub fn new_test_required(&self, time: Float) -> bool {
        let time_to_next_setup_change = self.initialization_time + 
            self.time_between_evaluations * self.test_index() as Float;
        
        if time > time_to_next_setup_change {
            true
        } else {
            false
        }
    }

    pub fn update_state_and_result_history(&mut self) { 
        if self.new_generation() {
             let (state, result) = if self.test_index() == 0 {
                let state = self.optimizer.initial_state();

                let result = SwarmResult::new(
                    self.optimizer.nr_particles,
                    self.optimizer.nr_dimensions
                );

                (state, result)
            } else {
                let previous_state = self.state_history.last().unwrap();
                let previous_result = self.result_history.last().unwrap();

                let state = self.optimizer.next_state(
                    &previous_state,
                    &previous_result,
                );

                let result = previous_result.next_initial_result();

                (state, result)
            };

            self.state_history.push(state);
            self.result_history.push(result);
        }
    }

    /// The function responsible for updating the optimizer with new simulation results.
    pub fn update(&mut self, time: &[Float], results: &[SimulationResult]) -> Option<ControllerOutput> {
        let current_time = time.last().unwrap().clone();

        if self.new_test_required(current_time) {
            self.update_state_and_result_history();

            let current_obj_func_value = self.objective_function(time, results);

            let history_index = self.state_history.len() - 1;

            let mut current_position = vec![0.0; self.optimizer.nr_dimensions];

            for i in 0..self.optimizer.nr_dimensions {
                current_position[i] = self.state_history[history_index].position[[self.current_particle, i]];
            }

            self.result_history[history_index].add_new_function_value(
                current_obj_func_value,
                self.current_particle,
                &current_position
            );

            let mut new_position = vec![0.0; self.optimizer.nr_dimensions];

            for i in 0..self.optimizer.nr_dimensions {
                new_position[i] = self.state_history[history_index].position[[self.current_particle, i]];
            }

            self.increase_test_index();

            Some(ControllerOutput {
                local_wing_angles: Some(new_position),
                section_models_internal_state: None,
            })
        } else {
            None
        }
    }
}