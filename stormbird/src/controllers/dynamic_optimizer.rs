
use stormath::optimization::particle_swarm::{
    SwarmState,
    SwarmResult,
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
    #[serde(default)]
    pub result_history: Vec<SwarmResult>,
}

impl DynamicOptimizer {
    pub fn objective_function(&self, _time: &[f64], _result: &[SimulationResult]) -> f64 {
        0.0
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
    pub fn new_test_required(&self, time: f64) -> bool {
        let time_to_next_setup_change = self.initialization_time + 
            self.time_between_evaluations * self.test_index() as f64;
        
        if time > time_to_next_setup_change {
            true
        } else {
            false
        }
    }

    /// The function responsible for updating the optimizer with new simulation results.
    pub fn update(&mut self, time: &[f64], results: &[SimulationResult]) -> Option<Vec<f64>> {
        let current_time = time.last().unwrap().clone();

        if self.new_test_required(current_time) {
            let current_obj_func_value = self.objective_function(time, results);

            let new_generation = self.new_generation();
            
        }

        return None;
    }
}