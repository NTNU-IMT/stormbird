

use crate::array2::Array2;

use serde::{Serialize, Deserialize};

use rand::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmState {
    /// The current iteration of the swarm
    pub iteration: usize,
    /// The current position of the particles in the swarm
    pub position: Array2<f64>,
    /// The velocity that brought the particles to their current position
    pub velocity: Array2<f64>,
}

impl SwarmState {
    pub fn nr_particles(&self) -> usize {
        self.position.nr_rows()
    }

    pub fn nr_dimensions(&self) -> usize {
        self.position.nr_cols()
    }

    pub fn local_best_position(&self, result: &SwarmResult) -> Vec<f64> {
        let best_index = result.local_best_index();

        let mut local_best_position = Vec::with_capacity(self.nr_dimensions());

        for j in 0..self.nr_dimensions() {
            local_best_position.push(self.position[[best_index, j]]);
        }

        local_best_position
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResult {
    pub function_values: Vec<f64>,
    pub global_best_position: Vec<f64>,
    pub global_best_function_value: f64,
}

impl SwarmResult {
    pub fn nr_particles(&self) -> usize {
        self.function_values.len()
    }

    pub fn local_best_index(&self) -> usize {
        let mut best_particle_index = 0;

        for i in 0.. self.nr_particles() {
            if self.function_values[i] < self.function_values[best_particle_index] {
                best_particle_index = i;
            }
        }

        best_particle_index
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSwarm {
    pub nr_particles: usize,
    pub nr_dimensions: usize,
    pub max_positions: Vec<f64>,
    pub min_positions: Vec<f64>,
    pub inertia_weight_max: f64,
    pub inertia_weight_min: f64,
    pub expected_number_of_generations: usize,
    pub local_best_velocity_factor: f64,
    pub global_best_velocity_factor: f64,
}

impl ParticleSwarm {
    pub fn initial_state(&self) -> SwarmState {
        let mut rng = rand::rng();

        let mut position = Array2::new_default(
            [self.nr_particles, self.nr_dimensions]
        );

        for i in 0..self.nr_particles {
            for j in 0..self.nr_dimensions {
                position[[i, j]] = rng.random_range(self.min_positions[j]..self.max_positions[j]);
            }
        }

        let mut velocity = Array2::new_default(
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

    pub fn velocity_scale(&self) -> Vec<f64> {
        let mut scale = Vec::with_capacity(self.nr_dimensions);

        for j in 0..self.nr_dimensions {
            scale.push(self.max_positions[j] - self.min_positions[j]);
        }

        scale
    }

    /// Computes a new velocities for the particles in the swarm, to be used to move them in the 
    /// next generation
    pub fn compute_velocity(&self, state: &SwarmState, result: &SwarmResult) -> Array2<f64> {
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

        let mut velocity = Array2::new_default(
            [self.nr_particles, self.nr_dimensions]
        );

        let inertia_decrease = (state.iteration as f64 / self.expected_number_of_generations as f64).min(1.0);
        let delta_inertia_weight = self.inertia_weight_max - self.inertia_weight_min;

        let inertia_weight = self.inertia_weight_max - delta_inertia_weight * inertia_decrease;

        let mut rng = rand::rng();

        let local_best_position = state.local_best_position(result);

        let velocity_scale = self.velocity_scale();
        
        for i in 0..self.nr_particles {
            for j in 0..self.nr_dimensions {
                let inertia_velocity = inertia_weight * state.velocity[[i, j]];

                let r_l = rng.random_range(0.0..self.local_best_velocity_factor); 
                let r_g = rng.random_range(0.0..self.global_best_velocity_factor); 

                let global_best_velocity = r_g * (result.global_best_position[j] - state.position[[i, j]]);
                let local_best_velocity = r_l * (local_best_position[j] - state.position[[i, j]]);

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