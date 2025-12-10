// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a complete simulation model for sails

use pyo3::prelude::*;

use stormbird::lifting_line::complete_sail_model::CompleteSailModel as CompleteSailModelRust;

use stormbird::wind::wind_condition::WindCondition;

use crate::result_structs::SimulationResult;

#[pyclass]
pub struct CompleteSailModel {
    data: CompleteSailModelRust
}

#[pymethods]
impl CompleteSailModel {
    #[new]
    pub fn new(
        setup_string: String
    ) -> Self {
        Self {
            data: CompleteSailModelRust::new_from_string(
                &setup_string
            ).unwrap()
        }
    }

    #[pyo3(signature=(
        *,
        time,
        time_step,
        wind_velocity,
        wind_direction,
        ship_velocity,
        controller_loading
    ))]
    pub fn do_step(
        &mut self,
        time: f64,
        time_step: f64,
        wind_velocity: f64,
        wind_direction: f64,
        ship_velocity: f64,
        controller_loading: f64
    ) -> SimulationResult {

        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction
        };

        let result_rs = self.data.do_step(
            time,
            time_step,
            wind_condition,
            ship_velocity,
            controller_loading,
        );

        SimulationResult {
            data: result_rs
        }
    }

    #[pyo3(signature=(
        *,
        wind_velocity,
        wind_direction,
        ship_velocity,
        controller_loading = 1.0,
        time_step = 1.0,
        nr_time_steps = 1
    ))]
    pub fn simulate_condition(
        &mut self,
        wind_velocity: f64,
        wind_direction: f64,
        ship_velocity: f64,
        controller_loading: f64,
        time_step: f64,
        nr_time_steps: usize
    ) -> SimulationResult {

        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction
        };

        let result_rs = self.data.simulate_condition(
            wind_condition,
            ship_velocity,
            controller_loading,
            time_step,
            nr_time_steps
        );

        SimulationResult {
            data: result_rs
        }
    }
    
    #[pyo3(signature=(
        *,
        wind_velocity,
        wind_direction,
        ship_velocity,
        nr_loadings_to_test = 10,
        time_step = 1.0,
        nr_time_steps = 1
    ))]
    pub fn simulate_condition_optimal_controller_loading(
        &mut self,
        wind_velocity: f64,
        wind_direction: f64,
        ship_velocity: f64,
        nr_loadings_to_test: usize,
        time_step: f64,
        nr_time_steps: usize
    ) -> SimulationResult {

        let wind_condition = WindCondition{
            velocity: wind_velocity,
            direction_coming_from: wind_direction
        };

        let result_rs = self.data.simulate_condition_optimal_controller_loading(
            wind_condition,
            ship_velocity,
            nr_loadings_to_test,
            time_step,
            nr_time_steps
        );

        SimulationResult {
            data: result_rs
        }
    }

    pub fn section_models_internal_state(&self) -> Vec<f64> {
        self.data.lifting_line_simulation.line_force_model.section_models_internal_state()
    }
}
