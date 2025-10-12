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

use crate::controllers::{
    Controller,
    input::ControllerInput,
};

use crate::common_utils::results::simulation::SimulationResult;

use builder::CompleteSailModelBuilder;

use stormath::spatial_vector::SpatialVector;

use stormath::type_aliases::Float;

use crate::error::Error;


#[derive(Debug, Clone)]
/// Collection of the necessary functionality to simulate a *complete* sail system using the lifting
/// line model. This means combining a lifting line model of the sails with a model of the wind 
/// conditions and a control system that adjust the control parameters of the sails based on the 
/// wind conditions.
pub struct CompleteSailModel {
    lifting_line_simulation: LiftingLineSimulation,
    wind_environment: WindEnvironment,
    controller: Controller
}

impl CompleteSailModel {
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let builder = CompleteSailModelBuilder::new_from_string(setup_string)?;

        Ok(builder.build())
    }

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

    /// Returns the forces on the sails
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

        let linear_velocity = SpatialVector::new(-ship_velocity, 0.0, 0.0);

        let freestream_velocity = self.wind_environment
            .apparent_wind_velocity_vectors_at_locations(
                wind_condition, 
                &freestream_velocity_points,
                linear_velocity
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