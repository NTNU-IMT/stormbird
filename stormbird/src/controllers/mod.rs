pub mod pid_controller;
pub mod particle_swarm_optimizer;
pub mod effective_angle_of_attack;

use crate::common_utils::results::simulation::SimulationResult;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineForceModelState {
    pub local_wing_angles: Vec<f64>,
    pub section_models_internal_state: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControllerOutput {
    pub local_wing_angles: Option<Vec<f64>>,
    pub section_models_internal_state: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControllerBuilder {
    EffectiveAngleOfAttack(effective_angle_of_attack::EffectiveAngleOfAttackControllerBuilder),
}

impl ControllerBuilder {
    pub fn build(&self) -> Controller {
        match self {
            ControllerBuilder::EffectiveAngleOfAttack(builder) => {
                Controller::EffectiveAngleOfAttack(builder.build())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Controller {
    EffectiveAngleOfAttack(effective_angle_of_attack::EffectiveAngleOfAttackController),
}


impl Controller {
    pub fn update(&mut self, time_step: f64, model_state: &LineForceModelState, simulation_result: &SimulationResult) -> Option<ControllerOutput> {
        match self {
            Controller::EffectiveAngleOfAttack(optimizer) => {
                optimizer.update(time_step, model_state, simulation_result)
            }
        }
    }
}
