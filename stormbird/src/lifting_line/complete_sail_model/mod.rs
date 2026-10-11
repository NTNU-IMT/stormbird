// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// The complete sail model combines a lifting line simulation with other functionality necessary 
/// for modeling a sail as a *complete unit*. 

pub mod builder;
pub mod settings;

use crate::lifting_line::simulation::Simulation as LiftingLineSimulation;
use crate::common_utils::forces_and_moments::IntegratedValues;

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
use settings::CompleteSailModelSettings;

use stormath::{
    type_aliases::Float,
    spatial_vector::SpatialVector,
    array_generation,
};
use crate::error::Error;

#[derive(Debug, Clone)]
/// Collection of the necessary functionality to simulate a *complete sail system* using the lifting
/// line model. This means combining a lifting line model of the sails with a model of the wind
/// environment and a control system that adjust the control parameters of the sails based on the
/// wind conditions.
pub struct CompleteSailModel {
    pub lifting_line_simulation: LiftingLineSimulation,
    pub wind_environment: WindEnvironment,
    pub controller: Controller,
    pub settings: CompleteSailModelSettings
}

impl CompleteSailModel {
    /// Generate a model from an input json string
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let builder = CompleteSailModelBuilder::new_from_string(setup_string)?;

        builder.build()
    }
    
    /// Query the model for the number of sails
    pub fn get_number_of_sails(&self) -> usize {
        self.lifting_line_simulation.line_force_model.nr_wings()
    }
    
    /// Runs multiple `simulate_steady_state_condition` calls with different loadings, and chooses 
    /// the best one based on the maximum delivered power
    pub fn simulate_optimal_steady_state_condition(
        &mut self,
        wind_condition: &WindCondition,
        ship_velocity: Float,
        max_loading: Float
    ) -> SimulationResult {

        let nr_loadings_to_test = self.settings.nr_loadings_to_test_during_optimization;
        
        let loadings_to_test = array_generation::linspace(0.0, max_loading, nr_loadings_to_test);
        
        let mut results: Vec<SimulationResult> = Vec::with_capacity(nr_loadings_to_test);
        let mut effective_power: Vec<Float> = Vec::with_capacity(nr_loadings_to_test);
        
        let mut max_effective_power = Float::NEG_INFINITY;
        let mut best_index = 0;
        
        for i in 0..nr_loadings_to_test {
            let result = self.simulate_steady_state_condition(
                wind_condition, 
                ship_velocity, 
                loadings_to_test[i]
            );
            
            let thrust = result.integrated_forces_sum().dot(self.settings.thrust_direction);
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
        
        let mut best_result = results[best_index].clone();

        // Set all integrated values to zero if the sails are not providing any useful work AND they
        // are set to be retractable
        if max_effective_power < 0.0 && self.settings.retractable {
            let nr_wings = best_result.nr_of_wings();
            
            best_result.input_power = vec![0.0; nr_wings];
            best_result.integrated_forces = vec![IntegratedValues::default(); nr_wings];
            best_result.integrated_moments = vec![IntegratedValues::default(); nr_wings];
        }

        best_result
    }
    
    /// The main interface to use the sail model in steady state conditions.
    /// 
    /// # Arguments
    /// - `wind_condition`: structure containing variables defining the wind condition.
    /// - `ship velocity`: The velocity of the ship in m/s
    /// - `controller_loading`: the loading of the controller, meaning how close to the max angle of 
    /// attack/max spin ratio/max flap angle/max suction rate the sail should be operated at. 1.0 
    /// means max values, while 0.0 means sails in neutral position 
    pub fn simulate_steady_state_condition(
        &mut self,
        wind_condition: &WindCondition,
        ship_velocity: Float,
        controller_loading: Float
    ) -> SimulationResult {
        let loading_used = controller_loading.min(1.0).max(0.0);

        // Apply initial controller settings based on the wind conditions alone, but which then 
        // neglects the lift-induced velocity.
        self.apply_controller_based_on_wind_condition(
            0.0, 
            1.0, 
            wind_condition, 
            ship_velocity, 
            loading_used
        );

        // Extract variables to assess convergence of the local wing angles
        let mut current_wing_angles = self.lifting_line_simulation
            .line_force_model.local_wing_angles.clone();

        let mut max_change_in_wing_angles = 999.0;
        let mut iteration = 0;
        let mut result: SimulationResult = SimulationResult::default();

        // Iterate until the local wing angles converge
        while iteration < self.settings.max_controller_iterations && max_change_in_wing_angles > self.settings.allowed_angle_error {
            result = self.do_step(
                iteration as Float,
                1.0,
                wind_condition, 
                ship_velocity
            );

            self.apply_controller_based_on_simulation_result(
                iteration as Float,
                1.0,
                loading_used,
                &result
            );

            let new_wing_angles = self.lifting_line_simulation
                .line_force_model.local_wing_angles.clone();

            let mut wing_angles_change = Vec::new();

            for (angle_new, angle_old) in new_wing_angles.iter().zip(current_wing_angles.iter()) {
                wing_angles_change.push((angle_new - angle_old).abs());
            }

            max_change_in_wing_angles = 0.0;

            for c in wing_angles_change.iter() {
                if *c > max_change_in_wing_angles {
                    max_change_in_wing_angles = *c;
                }
            }

            current_wing_angles = new_wing_angles;
            iteration += 1;
        }

        result    
    }
    
    pub fn simulate_steady_state_condition_simple_output(
        &mut self,
        wind_condition: &WindCondition,
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

    pub fn simulate_optimal_steady_state_condition_simple_output(
        &mut self,
        wind_condition: &WindCondition,
        ship_velocity: Float,
        max_loading: Float
    ) -> Vec<SingleSailResult> {
        let full_results = self.simulate_optimal_steady_state_condition(
            wind_condition, 
            ship_velocity,
            max_loading
        );
        
        full_results.as_simplified()
    }
    
    /// Simulate a condition for the sail, specified by a wind condition, ship velocity, 
    /// and controller loading
    pub fn do_multiple_steps(
        &mut self,
        end_time: Float,
        time_step: Float,
        wind_condition: &WindCondition,
        ship_velocity: Float,
    ) -> Vec<SimulationResult> {
        let mut results = Vec::new();

        self.lifting_line_simulation.first_time_step_completed = false; // Make sure the wake is re-initialized

        let mut current_time = 0.0;
        
        while current_time < end_time {
            results.push(
                self.do_step(
                    current_time,
                    time_step,
                    wind_condition,
                    ship_velocity,
                )
            );
            
            current_time += time_step;
        }
        
        results
    }

    /// Returns the forces on the sails for a single time step
    pub fn do_step(
        &mut self,
        current_time: Float,
        time_step: Float,
        wind_condition: &WindCondition,
        ship_velocity: Float
    ) -> SimulationResult {
        let freestream_velocity = self.freestream_velocity(
            wind_condition,
            ship_velocity,
            current_time
        );

        self.lifting_line_simulation.do_step(
            current_time,
            time_step,
            &freestream_velocity
        )
    }
    
    pub fn apply_controller_based_on_wind_condition(
        &mut self,
        current_time: Float,
        time_step: Float,
        wind_condition: &WindCondition,
        ship_velocity: Float,
        controller_loading: Float,
    ) {
        let freestream_velocity = self.freestream_velocity(
            wind_condition,
            ship_velocity,
            current_time
        );

        self.apply_controller_based_on_freestream(
            current_time,
            time_step,
            controller_loading,
            &freestream_velocity
        );
    }
    
    pub fn freestream_velocity(
        &self,
        wind_condition: &WindCondition,
        ship_velocity: Float,
        time: Float
    ) -> Vec<SpatialVector> {
        let freestream_velocity_points = self.lifting_line_simulation
            .get_freestream_velocity_points();

        let linear_velocity = ship_velocity * self.wind_environment.zero_direction_vector;
        
        let nr_points = freestream_velocity_points.len();
        
        let mut freestream_velocity = Vec::with_capacity(nr_points);
        
        for i in 0..nr_points {
            freestream_velocity.push(
                self.wind_environment.unsteady_apparent_wind_velocity_vector_at_location(
                    wind_condition, 
                    freestream_velocity_points[i], 
                    linear_velocity, 
                    time
                )
            );
        }
        
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
        let nr_ctrl_points = self.lifting_line_simulation.line_force_model.nr_span_lines();
        
        let ctrl_points_velocity = &freestream_velocity[0..nr_ctrl_points];
        
        let controller_input = ControllerInput::new_from_velocity(
            loading,
            &self.lifting_line_simulation.line_force_model,
            ctrl_points_velocity,
            &self.controller.spanwise_measurement,
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

    pub fn apply_controller_based_on_simulation_result(
        &mut self,
        current_time: Float,
        time_step: Float,
        loading: Float,
        simulation_result: &SimulationResult
    ) {
        
        let controller_input = ControllerInput::new_from_simulation_result(
            loading,
            &self.lifting_line_simulation.line_force_model,
            simulation_result,
            &self.controller.spanwise_measurement,
            &self.wind_environment,
            self.controller.use_input_velocity_for_apparent_wind_direction
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
    
    pub fn set_local_wing_angles(&mut self, local_wing_angles: &[Float]) {
        self.lifting_line_simulation
            .line_force_model
            .set_local_wing_angles(local_wing_angles);
    }
    
    pub fn set_section_models_internal_state(&mut self, internal_state: &[Float]) {
        self.lifting_line_simulation
            .line_force_model
            .set_section_models_internal_state(internal_state);
    }
}
