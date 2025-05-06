

use crate::array2::Array2;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmState {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSwarm {
    pub inertia_weight_max: f64,
    pub inertia_weight_min: f64,
    pub local_best_velocity_factor: f64,
    pub global_best_velocity_factor: f64,
}

impl ParticleSwarm {
    pub fn compute_velocity(&self, state: &SwarmState) -> Array2<f64> {
        let mut velocity = Array2::new_default(state.velocity.shape());

        let inertia_weight = 0.5 * (self.inertia_weight_max + self.inertia_weight_min); // TODO: make more clever algorithm for the inertia..
        
        for i in 0..state.nr_particles() {
            for j in 0..state.nr_dimensions() {
                let inertia = inertia_weight * state.velocity[[i, j]];

                let r_l = self.local_best_velocity_factor; // TODO: make this random
                let r_g = self.global_best_velocity_factor;

                let global_best = r_g * (state.global_best_position[j] - state.position[[i, j]]);

                velocity[[i, j]] = inertia + global_best;
            }
        }

        velocity
    }
}