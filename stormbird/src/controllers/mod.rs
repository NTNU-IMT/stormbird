// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of various controllers for wind propulsion devices intended to be used together
//! with the simulation models in the library. 

pub mod pid_controller;
pub mod particle_swarm_optimizer;
pub mod effective_angle_of_attack;

use crate::common_utils::results::simulation::SimulationResult;

use crate::io_utils::csv_data;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineForceModelState {
    pub local_wing_angles: Vec<f64>,
    pub section_models_internal_state: Vec<f64>,
}

impl LineForceModelState {
    pub fn as_csv_string(&self) -> (String, String) {
        let mut header = String::new();
        let mut data = String::new();

        for (i, angle) in self.local_wing_angles.iter().enumerate() {
            if i > 0 {
                header.push(',');
                data.push(',');
            }

            header.push_str(&format!("local_wing_angle_{}", i));
            data.push_str(&format!("{:.6}", angle));
        }

        for (i, state) in self.section_models_internal_state.iter().enumerate() {
            header.push(',');
            data.push(',');

            header.push_str(&format!("section_model_internal_state_{}", i));
            data.push_str(&format!("{:.6}", state));
        }

        (header, data)
    }

    pub fn write_to_csv_file(&self, file_path: &str) {
        let (header, data) = self.as_csv_string();

        let _ = csv_data::create_or_append_header_and_data_strings_file(
            file_path,
            &header,
            &data,
        );
    }
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
    pub fn update(
        &mut self,
        time: f64,
        time_step: f64, 
        model_state: &LineForceModelState, 
        simulation_result: &SimulationResult
    ) -> Option<ControllerOutput> {
        match self {
            Controller::EffectiveAngleOfAttack(optimizer) => {
                optimizer.update(time, time_step, model_state, simulation_result)
            }
        }
    }
}
