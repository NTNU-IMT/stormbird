// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a complete simulation model for sails

use pyo3::prelude::*;

use stormbird::lifting_line::complete_sail_model::CompleteSailModel as CompleteSailModelRust;
use stormath::spatial_vector::SpatialVector;

use crate::wind::wind_condition::WindCondition;

use crate::result_structs::{SimulationResult, SingleSailResult};

use crate::wind::environment::WindEnvironment;

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

    /// The number of sails in the model
    pub fn get_number_of_sails(&self) -> usize {
        self.data.get_number_of_sails()
    }

    /// Runs multiple steady state simulations with different controller loadings, and returns the
    /// result with the highest effective power. That is, the power delivered by the thrust, minus
    /// the power that is used by the sails. The number of loadings that are tested is defined by
    /// the model settings.
    #[pyo3(signature=(
        *,
        wind_condition,
        ship_velocity
    ))]
    pub fn simulate_optimal_steady_state_condition(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: f64
    ) -> SimulationResult {
        let result_rs = self.data.simulate_optimal_steady_state_condition(
            &wind_condition.data,
            ship_velocity
        );

        SimulationResult {
            data: result_rs
        }
    }

    /// Applies the controller and simulates a single steady state condition
    #[pyo3(signature=(
        *,
        wind_condition,
        ship_velocity,
        controller_loading
    ))]
    pub fn simulate_steady_state_condition(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: f64,
        controller_loading: f64
    ) -> SimulationResult {
        let result_rs = self.data.simulate_steady_state_condition(
            &wind_condition.data,
            ship_velocity,
            controller_loading
        );

        SimulationResult {
            data: result_rs
        }
    }

    /// Same as `simulate_steady_state_condition`, but the result is simplified to the total force
    /// and moment and the input power for each sail.
    #[pyo3(signature=(
        *,
        wind_condition,
        ship_velocity,
        controller_loading
    ))]
    pub fn simulate_steady_state_condition_simple_output(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: f64,
        controller_loading: f64
    ) -> Vec<SingleSailResult> {
        let results_rs = self.data.simulate_steady_state_condition_simple_output(
            &wind_condition.data,
            ship_velocity,
            controller_loading
        );

        results_rs.into_iter().map(
            |result| SingleSailResult { data: result }
        ).collect()
    }

    /// Same as `simulate_optimal_steady_state_condition`, but the result is simplified to the
    /// total force and moment and the input power for each sail.
    #[pyo3(signature=(
        *,
        wind_condition,
        ship_velocity
    ))]
    pub fn simulate_optimal_steady_state_condition_simple_output(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: f64
    ) -> Vec<SingleSailResult> {
        let results_rs = self.data.simulate_optimal_steady_state_condition_simple_output(
            &wind_condition.data,
            ship_velocity
        );

        results_rs.into_iter().map(
            |result| SingleSailResult { data: result }
        ).collect()
    }

    #[pyo3(signature=(
        *,
        time,
        time_step,
        wind_condition,
        ship_velocity,
        controller_loading
    ))]
    pub fn apply_controller_based_on_wind_condition(
        &mut self,
        time: f64,
        time_step: f64,
        wind_condition: WindCondition,
        ship_velocity: f64,
        controller_loading: f64
    ) {
        self.data.apply_controller_based_on_wind_condition(
            time,
            time_step,
            &wind_condition.data,
            ship_velocity,
            controller_loading
        );
    }

    /// The freestream velocity at all the points where it is needed by the simulation. That is,
    /// the apparent wind, including the effect of the ship velocity and inflow corrections.
    #[pyo3(signature=(
        *,
        wind_condition,
        ship_velocity,
        time
    ))]
    pub fn freestream_velocity(
        &self,
        wind_condition: WindCondition,
        ship_velocity: f64,
        time: f64
    ) -> Vec<[f64; 3]> {
        let freestream_velocity = self.data.freestream_velocity(
            &wind_condition.data,
            ship_velocity,
            time
        );

        freestream_velocity.iter().map(|v| v.0).collect()
    }

    #[pyo3(signature=(
        *,
        time,
        time_step,
        loading,
        freestream_velocity
    ))]
    pub fn apply_controller_based_on_freestream(
        &mut self,
        time: f64,
        time_step: f64,
        loading: f64,
        freestream_velocity: Vec<[f64; 3]>
    ) {
        let freestream_velocity_rs: Vec<SpatialVector> = freestream_velocity.iter().map(
            |v| SpatialVector::from(*v)
        ).collect();

        self.data.apply_controller_based_on_freestream(
            time,
            time_step,
            loading,
            &freestream_velocity_rs
        );
    }

    #[pyo3(signature=(
        *,
        time,
        time_step,
        loading,
        simulation_result
    ))]
    pub fn apply_controller_based_on_simulation_result(
        &mut self,
        time: f64,
        time_step: f64,
        loading: f64,
        simulation_result: SimulationResult
    ) {
        self.data.apply_controller_based_on_simulation_result(
            time,
            time_step,
            loading,
            &simulation_result.data
        );
    }

    #[pyo3(signature=(
        *,
        time,
        time_step,
        wind_condition,
        ship_velocity
    ))]
    pub fn do_step(
        &mut self,
        time: f64,
        time_step: f64,
        wind_condition: WindCondition,
        ship_velocity: f64
    ) -> SimulationResult {

        let result_rs = self.data.do_step(
            time,
            time_step,
            &wind_condition.data,
            ship_velocity
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
        ship_velocity
    ))]
    pub fn do_multiple_steps(
        &mut self,
        end_time: f64,
        time_step: f64,
        wind_condition: WindCondition,
        ship_velocity: f64
    ) -> Vec<SimulationResult> {

        let results_rs = self.data.do_multiple_steps(
            end_time,
            time_step,
            &wind_condition.data,
            ship_velocity
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

    pub fn local_wing_angles(&self) -> Vec<f64> {
        self.data.lifting_line_simulation.line_force_model.local_wing_angles.clone()
    }

    pub fn set_local_wing_angles(&mut self, local_wing_angles: Vec<f64>) {
        self.data.set_local_wing_angles(&local_wing_angles);
    }

    pub fn set_section_models_internal_state(&mut self, internal_state: Vec<f64>) {
        self.data.set_section_models_internal_state(&internal_state);
    }

    pub fn get_wind_environment(&self) -> WindEnvironment {
        WindEnvironment{
            data: self.data.wind_environment.clone()
        }
    }
}
