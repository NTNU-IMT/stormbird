// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a complete simulation model for sails

use pyo3::prelude::*;

use stormbird::lifting_line::complete_sail_model::CompleteSailModel as CompleteSailModelRust;

use crate::wind::wind_condition::WindCondition;

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
        wind_condition,
        ship_velocity,
        controller_loading
    ))]
    pub fn do_step(
        &mut self,
        time: f64,
        time_step: f64,
        wind_condition: WindCondition,
        ship_velocity: f64,
        controller_loading: f64
    ) -> SimulationResult {

        let result_rs = self.data.do_step(
            time,
            time_step,
            &wind_condition.data,
            ship_velocity,
            controller_loading,
        );

        SimulationResult {
            data: result_rs
        }
    }

    #[pyo3(signature=(
        *,
        end_time,
        time_step,
        wind_condition,
        ship_velocity,
        controller_loading    
    ))]
    pub fn do_multiple_steps(
        &mut self,
        end_time: f64,
        time_step: f64,
        wind_condition: WindCondition,
        ship_velocity: f64,
        controller_loading: f64,
        
    ) -> Vec<SimulationResult> {

        let results_rs = self.data.do_multiple_steps(
            end_time,
            time_step,
            &wind_condition.data,
            ship_velocity,
            controller_loading
        );
        
        let mut out = Vec::with_capacity(results_rs.len());
        
        for i in 0..results_rs.len() {
            out.push(
                SimulationResult {
                    data: results_rs[i].clone()
                }
            )
        }
        
        out
    }

    pub fn section_models_internal_state(&self) -> Vec<f64> {
        self.data.lifting_line_simulation.line_force_model.section_models_internal_state()
    }
}
