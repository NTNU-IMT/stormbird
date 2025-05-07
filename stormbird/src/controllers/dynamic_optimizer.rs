
use stormath::optimization::particle_swarm::{
    SwarmState,
    ParticleSwarm
};

use crate::common_utils::result::SimulationResult;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// This structure is used to control an optimization process that can run in dynamic simulations. 
pub struct DynamicOptimizer {
    pub initialization_time: f64,
    pub time_between_evaluations: f64,
    pub averaging_time: f64,
    pub optimizer: ParticleSwarm,
    #[serde(default)]
    pub current_particle: usize,
    #[serde(default)]
    pub current_generation: usize,
    #[serde(default)]
    pub state_history: Vec<SwarmState>,
}

impl DynamicOptimizer {
    pub fn objective_function(&self, _result: &SimulationResult) -> f64 {
        0.0
    }

    pub fn update(&mut self, _result: &SimulationResult, time: f64) -> Option<Vec<f64>> {
        if time < self.initialization_time {
            return None;
        }

        return None;
    }
}