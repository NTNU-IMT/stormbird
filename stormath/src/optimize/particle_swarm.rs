// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Particle swarm optimization, for finding the minimum of a function within a bounded domain.
//!
//! Source: <https://en.wikipedia.org/wiki/Particle_swarm_optimization>

use crate::matrix::Matrix;

use serde::{Serialize, Deserialize};
use crate::type_aliases::Float;
use crate::consts::INFINITY;

use rand::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The state of the particles in a particle swarm, for one generation
pub struct SwarmState {
    /// The current iteration of the swarm
    pub iteration: usize,
    /// The current position of the particles in the swarm
    pub position: Matrix<Float>,
    /// The velocity that brought the particles to their current position
    pub velocity: Matrix<Float>,
}

impl SwarmState {
    /// Returns the number of particles in the swarm
    pub fn nr_particles(&self) -> usize {
        self.position.nr_rows()
    }

    /// Returns the number of dimensions of the search domain
    pub fn nr_dimensions(&self) -> usize {
        self.position.nr_cols()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The function values for the particles in a particle swarm, together with the best positions
/// found so far
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
    /// Creates a new result, where all function values are set to infinity
    pub fn new(nr_particles: usize, nr_dimensions: usize) -> Self {
        let mut function_values = Vec::with_capacity(nr_particles);
        let global_best_position = vec![0.0; nr_dimensions];
        let local_best_positions = Matrix::new_default(
            [nr_particles, nr_dimensions]
        );
        let mut local_best_function_values = Vec::with_capacity(nr_particles);

        for _ in 0..nr_particles {
            function_values.push(INFINITY);
            local_best_function_values.push(INFINITY);   
        }

        SwarmResult {
            function_values,
            local_best_positions,
            local_best_function_values,
            global_best_position,
            global_best_function_value: INFINITY,
        }
    }

    /// Returns the number of particles in the swarm
    pub fn nr_particles(&self) -> usize {
        self.function_values.len()
    }

    /// Returns the result to use for the next generation. The best positions and function values
    /// are kept, while the function values for the current generation are reset to infinity.
    pub fn next_initial_result(&self) -> SwarmResult {
        let mut next_result = self.clone();

        for i in 0..self.nr_particles() {
            next_result.function_values[i] = INFINITY;
        }

        next_result
    }

    /// Stores the function value for a particle, and updates the local and global best positions
    /// if the value is lower than the previous best values
    pub fn add_new_function_value(
        &mut self, 
        function_value: Float, 
        particle_index: usize, 
        particle_position: &[Float]
    ) {
        self.function_values[particle_index] = function_value;

        if function_value < self.local_best_function_values[particle_index] {
            self.local_best_function_values[particle_index] = function_value;

            let nr_cols = self.local_best_positions.nr_cols();

            for (j, &position) in particle_position.iter().take(nr_cols).enumerate() {
                self.local_best_positions[[particle_index, j]] = position;
            }
        }
        
        if function_value < self.global_best_function_value {
            self.global_best_function_value = function_value;
           
            let nr_dimensions = self.global_best_position.len();

            self.global_best_position.copy_from_slice(&particle_position[..nr_dimensions]);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Settings for a particle swarm optimization
pub struct ParticleSwarm {
    /// Number of particles in the swarm
    pub nr_particles: usize,
    /// Number of dimensions of the search domain
    pub nr_dimensions: usize,
    /// Expected number of generations. Used to decrease the inertia weight over the generations.
    pub expected_number_of_generations: usize,
    /// Upper bound of the search domain in each dimension
    pub max_positions: Vec<Float>,
    /// Lower bound of the search domain in each dimension
    pub min_positions: Vec<Float>,
    /// Inertia weight at the first generation
    #[serde(default = "ParticleSwarm::default_inertia_weight_max")]
    pub inertia_weight_max: Float,
    /// Inertia weight at the expected last generation
    #[serde(default = "ParticleSwarm::default_inertia_weight_min")]
    pub inertia_weight_min: Float,
    /// Maximum factor for the velocity towards the best position of each particle
    #[serde(default = "ParticleSwarm::default_local_best_velocity_factor")]
    pub local_best_velocity_factor: Float,
    /// Maximum factor for the velocity towards the best position of the whole swarm
    #[serde(default = "ParticleSwarm::default_global_best_velocity_factor")]
    pub global_best_velocity_factor: Float,
}

impl ParticleSwarm {
    fn default_inertia_weight_max() -> Float {1.0}
    fn default_inertia_weight_min() -> Float {0.4}
    fn default_local_best_velocity_factor() -> Float {2.0}
    fn default_global_best_velocity_factor() -> Float {2.0}

    /// Returns an initial state with random positions and velocities within the search domain
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

        for i in 0..self.nr_particles {
            for j in 0..self.nr_dimensions {
                let inertia_velocity = inertia_weight * state.velocity[[i, j]];

                let r_l = rng.random_range(0.0..self.local_best_velocity_factor); 
                let r_g = rng.random_range(0.0..self.global_best_velocity_factor); 

                let global_best_velocity = r_g * (result.global_best_position[j] - state.position[[i, j]]);
                let local_best_velocity = r_l * (result.local_best_positions[[i, j]] - state.position[[i, j]]);

                velocity[[i, j]] = inertia_velocity + global_best_velocity + local_best_velocity;
            }
        }

        velocity
    }

    /// Returns the state of the next generation, where the particles are moved with new velocities
    /// and limited to the search domain
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_function(position: &[Float]) -> Float {
        (position[0] - 1.0).powi(2) + (position[1] + 0.5).powi(2)
    }

    #[test]
    fn test_particle_swarm() {
        let swarm = ParticleSwarm {
            nr_particles: 30,
            nr_dimensions: 2,
            expected_number_of_generations: 50,
            max_positions: vec![2.0, 2.0],
            min_positions: vec![-2.0, -2.0],
            inertia_weight_max: ParticleSwarm::default_inertia_weight_max(),
            inertia_weight_min: ParticleSwarm::default_inertia_weight_min(),
            local_best_velocity_factor: ParticleSwarm::default_local_best_velocity_factor(),
            global_best_velocity_factor: ParticleSwarm::default_global_best_velocity_factor(),
        };

        let mut state = swarm.initial_state();
        let mut result = SwarmResult::new(swarm.nr_particles, swarm.nr_dimensions);

        for _ in 0..swarm.expected_number_of_generations {
            for i in 0..swarm.nr_particles {
                let position = [state.position[[i, 0]], state.position[[i, 1]]];

                for j in 0..swarm.nr_dimensions {
                    assert!(
                        position[j] >= swarm.min_positions[j] && 
                        position[j] <= swarm.max_positions[j],
                        "Position outside bounds: {:?}", position
                    );
                }

                result.add_new_function_value(test_function(&position), i, &position);
            }

            state = swarm.next_state(&state, &result);
            result = result.next_initial_result();
        }

        let best_value = test_function(&result.global_best_position);

        assert_eq!(best_value, result.global_best_function_value);
        assert!(best_value < 0.01, "best value: {}", best_value);
    }
}
