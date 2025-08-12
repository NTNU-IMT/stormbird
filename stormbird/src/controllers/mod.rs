// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of various controllers for wind propulsion devices intended to be used together
//! with the simulation models in the library. 

pub mod input;
pub mod output;
pub mod measurements;
pub mod weather_dependent_set_points;
pub mod effective_angle_of_attack;
pub mod pid_controller;

pub mod prelude;

use crate::common_utils::results::simulation::SimulationResult;

use serde::{Deserialize, Serialize};

use input::ControllerInput;
use output::ControllerOutput;

use effective_angle_of_attack::{
    EffectiveAngleOfAttackController,
    EffectiveAngleOfAttackControllerBuilder
};

use weather_dependent_set_points::{
    WeatherDependentSetPoints
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControllerBuilder {
    WeatherDependentSetPoints(WeatherDependentSetPoints),
    EffectiveAngleOfAttack(EffectiveAngleOfAttackControllerBuilder),
}

impl ControllerBuilder {
    pub fn build(&self) -> Controller {
        match self {
            ControllerBuilder::EffectiveAngleOfAttack(builder) => {
                Controller::EffectiveAngleOfAttack(builder.build())
            },
            ControllerBuilder::WeatherDependentSetPoints(controller) => {
                Controller::WeatherDependentSetPoints(controller.clone())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Controller {
    EffectiveAngleOfAttack(EffectiveAngleOfAttackController),
    WeatherDependentSetPoints(WeatherDependentSetPoints),
}


impl Controller {
    pub fn update(
        &mut self,
        time: f64,
        time_step: f64, 
        input: &ControllerInput,
    ) -> Option<ControllerOutput> {
        match self {
            Controller::EffectiveAngleOfAttack(controller) => {
                controller.update(time, time_step, input)
            },
            Controller::WeatherDependentSetPoints(controller) => {
                controller.update(time, time_step, input)
            }
        }
    }
}
