// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// The simple sail model is a handy way to use the Stormbird library when the goal is to model
/// sails in a simple and straightforward way. For instance, it can be to quickly set up a model of
/// a generic sail type, where the exact details are not that important.

pub mod builder;

use crate::lifting_line::simulation::Simulation as LiftingLineSimulation;

use crate::wind::{
    environment::WindEnvironment,
    wind_condition::WindCondition
};

use crate::controller::{
    Controller,
    input::ControllerInput,
};

use crate::common_utils::results::{
    simulation::SimulationResult,
    simplfied::SingleSailResult,
};

use builder::CompleteSailModelBuilder;

use stormath::{
    type_aliases::Float,
    spatial_vector::SpatialVector,
    array_generation,
};
use crate::error::Error;

#[derive(Debug, Clone)]
/// Collection of the necessary functionality to simulate a *complete sail system* using the lifting
/// line model. This means combining a lifting line model of the sails with a model of the wind
/// conditions and a control system that adjust the control parameters of the sails based on the
/// wind conditions.
pub struct CompleteSailModel {
    pub lifting_line_simulation: LiftingLineSimulation,
    pub wind_environment: WindEnvironment,
    pub controller: Controller,
}

impl CompleteSailModel {
    /// Generate a model from an input json string
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let builder = CompleteSailModelBuilder::new_from_string(setup_string)?;

        Ok(builder.build())
    }
    
    /// Query the model for the number of sails
    pub fn get_number_of_sails(&self) -> usize {
        self.lifting_line_simulation.line_force_model.nr_wings()
    }
    
    /// Runs multiple `simulate_condition` calls with different loadings, and chooses the best one
    /// based on the maximum delivered power
    pub fn simulate_condition_optimal_controller_loading(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: Float,
        nr_loadings_to_test: usize,
        time_step: Float,
        nr_time_steps: usize,
    ) -> SimulationResult {
        
        let loadings_to_test = array_generation::linspace(0.1, 1.0, nr_loadings_to_test);
        
        let mut results: Vec<SimulationResult> = Vec::with_capacity(nr_loadings_to_test);
        let mut effective_power: Vec<f64> = Vec::with_capacity(nr_loadings_to_test);
        
        let mut max_effective_power = Float::NEG_INFINITY;
        let mut best_index = 0;
        
        for i in 0..nr_loadings_to_test {
            let result = self.simulate_condition(
                wind_condition, 
                ship_velocity, 
                loadings_to_test[i], 
                time_step, 
                nr_time_steps
            );
            
            // TODO: must find a way to define what the thrust direction is!
            let thrust = -result.integrated_forces_sum()[0];
            let delivered_power = thrust * ship_velocity;
            let input_power = result.input_power_sum();
            
            effective_power.push(delivered_power - input_power);
            
            if effective_power[i] > max_effective_power {
                max_effective_power = effective_power[i];
                best_index = i;
            }
            
            results.push(
              result  
            );
        }
        
        results[best_index].clone()
    }
    
    pub fn simulate_steady_state_condition(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: Float,
        controller_loading: Float
    ) -> SimulationResult {
        self.simulate_condition(
            wind_condition, 
            ship_velocity, 
            controller_loading, 
            1.0, 
            1
        )
    }
    
    pub fn simulate_steady_state_condition_simple_output(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: Float,
        controller_loading: Float
    ) -> Vec<SingleSailResult> {
        let full_results = self.simulate_steady_state_condition(
            wind_condition, 
            ship_velocity, 
            controller_loading
        );
        
        full_results.as_simplified()
    }
    
    /// Simulate a condition for the sail, specified by a wind condition, ship velocity, 
    /// and controller loading
    pub fn simulate_condition(
        &mut self,
        wind_condition: WindCondition,
        ship_velocity: Float,
        controller_loading: Float,
        time_step: Float,
        nr_time_steps: usize,
    ) -> SimulationResult {
        let mut result = SimulationResult::default();

        self.lifting_line_simulation.first_time_step_completed = false; // Make sure the wake is re-initialized

        for time_index in 0..nr_time_steps {
            let current_time = (time_index as Float) * time_step;

            result = self.do_step(
                current_time,
                time_step,
                wind_condition,
                ship_velocity,
                controller_loading
            );
        }

        result
    }

    /// Returns the forces on the sails for a single time step
    pub fn do_step(
        &mut self,
        current_time: Float,
        time_step: Float,
        wind_condition: WindCondition,
        ship_velocity: Float,
        controller_loading: Float,
    ) -> SimulationResult {
        let freestream_velocity = self.freestream_velocity(
            wind_condition,
            ship_velocity
        );

        self.apply_controller_based_on_freestream(
            current_time,
            time_step,
            controller_loading,
            &freestream_velocity
        );

        self.lifting_line_simulation.do_step(
            current_time,
            time_step,
            &freestream_velocity
        )
    }
    
    pub fn freestream_velocity(
        &self,
        wind_condition: WindCondition,
        ship_velocity: Float
    ) -> Vec<SpatialVector> {
        let freestream_velocity_points = self.lifting_line_simulation
            .get_freestream_velocity_points();

        let linear_velocity = ship_velocity * self.wind_environment.zero_direction_vector;
        
        let mut freestream_velocity = self.wind_environment.apparent_wind_velocity_vectors_at_locations(
            wind_condition, 
            &freestream_velocity_points, 
            linear_velocity
        );
        
        let reference_height = 10.0;
        
        let apparent_wind_direction = self.wind_environment
            .apparent_wind_direction_from_condition_and_linear_velocity(
                wind_condition,
                linear_velocity,
                reference_height
            );
        
        self.wind_environment.apply_inflow_corrections(
            apparent_wind_direction,
            &mut freestream_velocity,
            &self.lifting_line_simulation.line_force_model.ctrl_points_global,
            &self.lifting_line_simulation.line_force_model.wing_indices
        );

        freestream_velocity
    }

    pub fn apply_controller_based_on_freestream(
        &mut self,
        current_time: Float,
        time_step: Float,
        loading: Float,
        freestream_velocity: &[SpatialVector]
    ) {
        let controller_input = ControllerInput::new_from_velocity(
            loading,
            &self.lifting_line_simulation.line_force_model,
            freestream_velocity,
            &self.controller.flow_measurement_settings,
            &self.wind_environment,
        );

        let controller_output = self.controller.update(
            current_time,
            time_step,
            &controller_input
        );

        if let Some(output) = &controller_output {
            self.lifting_line_simulation.line_force_model.set_controller_output(
                output
            );
        }
    }
}
