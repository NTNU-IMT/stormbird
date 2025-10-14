// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use crate::matrix::Matrix;

use serde::{Serialize, Deserialize};
use crate::type_aliases::Float;
use crate::consts::INFINITY;

use rand::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmState {
    /// The current iteration of the swarm
    pub iteration: usize,
    /// The current position of the particles in the swarm
    pub position: Matrix<Float>,
    /// The velocity that brought the particles to their current position
    pub velocity: Matrix<Float>,
}

impl SwarmState {
    pub fn nr_particles(&self) -> usize {
        self.position.nr_rows()
    }

    pub fn nr_dimensions(&self) -> usize {
        self.position.nr_cols()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResult {
    /// The function values of the particles in the swarm
    pub function_values: Vec<Float>,
    /// The historical best position of each particle in the swarm
    pub local_best_positions: Matrix<Float>,
    /// The historical best function value of each particle in the swarm
    pub local_best_function_values: Vec<Float>,
    /// The historical best position of all the particles in the swarm
    pub global_best_position: Vec<Float>,
    /// The historical best function value of the particles in the swarm
    pub global_best_function_value: Float,
}

impl SwarmResult {
    pub fn new(nr_particles: usize, nr_dimensions: usize) -> Self {
        let mut function_values = Vec::with_capacity(nr_particles);
        let mut global_best_position = Vec::with_capacity(nr_dimensions);
        let local_best_positions = Matrix::new_default(
            [nr_particles, nr_dimensions]
        );
        let mut local_best_function_values = Vec::with_capacity(nr_particles);

        for _ in 0..nr_particles {
            function_values.push(INFINITY);
            local_best_function_values.push(INFINITY);   
        }

        for _ in 0..nr_dimensions {
            global_best_position.push(0.0);
        }

        SwarmResult {
            function_values,
            local_best_positions,
            local_best_function_values,
            global_best_position,
            global_best_function_value: INFINITY,
        }
    }

    pub fn nr_particles(&self) -> usize {
        self.function_values.len()
    }

    pub fn next_initial_result(&self) -> SwarmResult {
        let mut next_result = self.clone();

        for i in 0..self.nr_particles() {
            next_result.function_values[i] = INFINITY;
        }

        next_result
    }

    pub fn add_new_function_value(
        &mut self, 
        function_value: Float, 
        particle_index: usize, 
        particle_position: &[Float]
    ) {
        self.function_values[particle_index] = function_value;

        if function_value < self.local_best_function_values[particle_index] {
            self.local_best_function_values[particle_index] = function_value;

            for j in 0..self.local_best_positions.nr_cols() {
                self.local_best_positions[[particle_index, j]] = particle_position[j];
            }
        }
        
        if function_value < self.global_best_function_value {
            self.global_best_function_value = function_value;
           
            for j in 0..self.global_best_position.len() {
                self.global_best_position[j] = particle_position[j];
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSwarm {
    pub nr_particles: usize,
    pub nr_dimensions: usize,
    pub expected_number_of_generations: usize,
    pub max_positions: Vec<Float>,
    pub min_positions: Vec<Float>,
    #[serde(default = "ParticleSwarm::default_inertia_weight_max")]
    pub inertia_weight_max: Float,
    #[serde(default = "ParticleSwarm::default_inertia_weight_min")]
    pub inertia_weight_min: Float,
    #[serde(default = "ParticleSwarm::default_local_best_velocity_factor")]
    pub local_best_velocity_factor: Float,
    #[serde(default = "ParticleSwarm::default_global_best_velocity_factor")]
    pub global_best_velocity_factor: Float,
}

impl ParticleSwarm {
    fn default_inertia_weight_max() -> Float {1.0}
    fn default_inertia_weight_min() -> Float {0.4}
    fn default_local_best_velocity_factor() -> Float {2.0}
    fn default_global_best_velocity_factor() -> Float {2.0}

    pub fn initial_state(&self) -> SwarmState {
        let mut rng = rand::rng();

        let mut position = Matrix::new_default(
            [self.nr_particles, self.nr_dimensions]
        );

        for i in 0..self.nr_particles {
            for j in 0..self.nr_dimensions {
                position[[i, j]] = rng.random_range(self.min_positions[j]..self.max_positions[j]);
            }
        }

        let mut velocity = Matrix::new_default(
            [self.nr_particles, self.nr_dimensions]
        );

        for i in 0..self.nr_particles {
            for j in 0..self.nr_dimensions {
                velocity[[i, j]] = rng.random_range(self.min_positions[j]..self.max_positions[j]);
            }
        }

        SwarmState {
            iteration: 0,
            position,
            velocity,
        }
    }

    pub fn velocity_scale(&self) -> Vec<Float> {
        let mut scale = Vec::with_capacity(self.nr_dimensions);

        for j in 0..self.nr_dimensions {
            scale.push(self.max_positions[j] - self.min_positions[j]);
        }

        scale
    }

    /// Computes a new velocities for the particles in the swarm, to be used to move them in the 
    /// next generation
    pub fn compute_velocity(&self, state: &SwarmState, result: &SwarmResult) -> Matrix<Float> {
        if state.nr_particles() != self.nr_particles {
            panic!(
                "Particle swarm size mismatch: expected {}, got {}", 
                self.nr_particles, state.nr_particles()
            );
        }

        if state.nr_dimensions() != self.nr_dimensions {
            panic!(
                "Particle swarm dimension mismatch: expected {}, got {}", 
                self.nr_dimensions, state.nr_dimensions()
            );
        }

        let mut velocity = Matrix::new_default(
            [self.nr_particles, self.nr_dimensions]
        );

        let inertia_decrease = (state.iteration as Float / self.expected_number_of_generations as Float).min(1.0);
        let delta_inertia_weight = self.inertia_weight_max - self.inertia_weight_min;

        let inertia_weight = self.inertia_weight_max - delta_inertia_weight * inertia_decrease;

        let mut rng = rand::rng();

        let velocity_scale = self.velocity_scale();
        
        for i in 0..self.nr_particles {
            for j in 0..self.nr_dimensions {
                let inertia_velocity = inertia_weight * state.velocity[[i, j]];

                let r_l = rng.random_range(0.0..self.local_best_velocity_factor); 
                let r_g = rng.random_range(0.0..self.global_best_velocity_factor); 

                let global_best_velocity = r_g * (result.global_best_position[j] - state.position[[i, j]]);
                let local_best_velocity = r_l * (result.local_best_positions[[i, j]] - state.position[[i, j]]);

                velocity[[i, j]] = inertia_velocity + (
                    global_best_velocity + local_best_velocity
                ) * velocity_scale[j];
            }
        }

        velocity
    }

    pub fn next_state(
        &self, 
        current_state: &SwarmState, 
        current_result: &SwarmResult
    ) -> SwarmState {
        let velocity = self.compute_velocity(current_state, current_result);

        let mut positions = current_state.position.clone() + velocity.clone();

        for i in 0..self.nr_particles {
            for j in 0..self.nr_dimensions {
                if positions[[i, j]] > self.max_positions[j] {
                    positions[[i, j]] = self.max_positions[j];
                } else if positions[[i, j]] < self.min_positions[j] {
                    positions[[i, j]] = self.min_positions[j];
                }
            }
        }

        SwarmState {
            iteration: current_state.iteration + 1,
            position: positions,
            velocity,
        }        
    }
}