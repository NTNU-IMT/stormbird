

use crate::array2::Array2;

use serde::{Serialize, Deserialize};

use rand::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmState {
    pub iteration: usize,
    pub position: Array2<f64>,
    pub velocity: Array2<f64>,
    pub function_values: Vec<f64>,
    pub global_best_position: Vec<f64>,
    pub global_best_function_value: f64,
}

impl SwarmState {
    pub fn nr_particles(&self) -> usize {
        self.position.nr_rows()
    }

    pub fn nr_dimensions(&self) -> usize {
        self.position.nr_cols()
    }

    pub fn local_best_position(&self) -> Vec<f64> {
        let mut best_particle_index = 0;

        for i in 0.. self.nr_particles() {
            if self.function_values[i] < self.function_values[best_particle_index] {
                best_particle_index = i;
            }
        }

        let mut local_best_position = Vec::with_capacity(self.nr_dimensions());
        for j in 0..self.nr_dimensions() {
            local_best_position.push(self.position[[best_particle_index, j]]);
        }

        local_best_position
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSwarm {
    pub inertia_weight_max: f64,
    pub inertia_weight_min: f64,
    pub expected_number_of_steps: usize,
    pub local_best_velocity_factor: f64,
    pub global_best_velocity_factor: f64,
}

impl ParticleSwarm {
    pub fn compute_velocity(&self, state: &SwarmState) -> Array2<f64> {
        let mut velocity = Array2::new_default(state.velocity.shape());

        let inertia_decrease = (state.iteration as f64 / self.expected_number_of_steps as f64).min(1.0);
        let delta_inertia_weight = self.inertia_weight_max - self.inertia_weight_min;

        let inertia_weight = self.inertia_weight_max - delta_inertia_weight * inertia_decrease;

        let mut rng = rand::rng();

        let local_best_position = state.local_best_position();
        
        for i in 0..state.nr_particles() {
            for j in 0..state.nr_dimensions() {
                let inertia_velocity = inertia_weight * state.velocity[[i, j]];

                let r_l = rng.random_range(0.0..self.local_best_velocity_factor); 
                let r_g = rng.random_range(0.0..self.global_best_velocity_factor); 

                let global_best_velocity = r_g * (state.global_best_position[j] - state.position[[i, j]]);
                let local_best_velocity = r_l * (local_best_position[j] - state.position[[i, j]]);

                velocity[[i, j]] = inertia_velocity + global_best_velocity + local_best_velocity;
            }
        }

        velocity
    }
}