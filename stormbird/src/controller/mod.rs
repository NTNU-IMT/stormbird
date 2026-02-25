// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of a controller for wind propulsion devices intended to be used together
//! with the simulation models in the library. 

pub mod builder;
pub mod input;
pub mod output;
pub mod measurements;
pub mod logic;
pub mod prelude;

use input::ControllerInput;
use output::ControllerOutput;
use logic::ControllerLogic;
use measurements::FlowMeasurementSettings;

use stormath::type_aliases::Float;

#[derive(Debug, Clone)]
pub struct Controller {
    /// Vector containing the set points for all the sails
    pub logic: ControllerLogic,
    /// Structure defining how to measure the representative flow conditions on the sail
    pub flow_measurement_settings: FlowMeasurementSettings,
    /// How often to update the controller
    pub time_steps_between_updates: usize,
    /// When to start using the controller
    pub start_time: Float,
    /// Limits on the change rate of the angle of attack
    pub max_local_wing_angle_change_rate: Option<Float>,
    /// Limits on the change rate of the internal state
    pub max_internal_section_state_change_rate: Option<Float>,
    /// Internal variable to keep track of the number of time steps executed
    pub time_step_index: usize,
    /// Switch to determine which velocity to use when measuring the apparent wind direction
    pub use_input_velocity_for_apparent_wind_direction: bool,
}

impl Controller {
    pub fn update(
        &self,
        time: Float,
        time_step: Float, 
        input: &ControllerInput,
    ) -> Option<ControllerOutput> {
        let initialization_done = time >= self.start_time;
        let time_to_update =  self.time_step_index % self.time_steps_between_updates == 0;
        let first_time_step = self.time_step_index == 1;

        if first_time_step || (time_to_update && initialization_done) {
            let mut output = self.logic.get_new_output(input);

            if self.max_local_wing_angle_change_rate.is_some() && output.local_wing_angles.is_some() {
                let raw_new_values = output.local_wing_angles.as_ref().unwrap();
                let max_change = self.max_local_wing_angle_change_rate.unwrap() * time_step;

                output.local_wing_angles = Some(
                    output::limit_values(
                        &input.current_local_wing_angles, 
                        raw_new_values, 
                        max_change
                    )
                )
            }

            if self.max_local_wing_angle_change_rate.is_some() && output.section_models_internal_state.is_some() {
                let raw_new_values = output.section_models_internal_state.as_ref().unwrap();
                let max_change = self.max_internal_section_state_change_rate.unwrap() * time_step;

                output.section_models_internal_state = Some(
                    output::limit_values(
                        &input.current_section_models_internal_state, 
                        raw_new_values, 
                        max_change
                    )
                )
            }

            return Some(output)
        }

        None
    }
}


