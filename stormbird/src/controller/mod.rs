// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of a controller for wind propulsion devices intended to be used together
//! with the simulation models in the library. 

/// Builder for the controller
pub mod builder;
/// Input to the controller
pub mod input;
/// Output from the controller
pub mod output;
/// Measurement of representative flow conditions on the sails
pub mod measurements;
/// Moving average filter for the input to the controller
pub mod input_filter;
/// Set points for the controller
pub mod set_points;
/// Re-exports of commonly used structures, to simplify imports
pub mod prelude;

use crate::line_force_model::LineForceModel;
use input::ControllerInput;
use output::ControllerOutput;
use set_points::ControllerSetPoints;
use measurements::SpanwiseMeasurement;
use input_filter::ControllerInputFilter;

use stormath::type_aliases::Float;

#[derive(Debug, Clone)]
/// A controller that sets the local wing angles and internal states of the sails, based on the
/// measured flow conditions and the set points for each sail
pub struct Controller {
    /// Vector containing the set points for all the sails
    pub set_points: Vec<ControllerSetPoints>,
    /// Structure defining how to measure the representative flow conditions on the sail
    pub spanwise_measurement: SpanwiseMeasurement,
    /// How often to update the controller
    pub time_steps_between_updates: usize,
    /// When to start using the controller
    pub start_time: Float,
    /// Internal variable to keep track of the number of time steps executed
    pub time_step_index: usize,
    /// Switch to determine which velocity to use when measuring the apparent wind direction
    pub use_input_velocity_for_apparent_wind_direction: bool,
    /// Optional moving average filters for the input, one for each wing
    pub input_filters: Option<Vec<ControllerInputFilter>>,
}

impl Controller {
    /// Returns the new local wing angle for each wing
    pub fn get_local_wing_angles(
        &self,
        time_step: Float,
        inputs: &[ControllerInput],
        line_force_model: &LineForceModel,
    ) -> Vec<Float> {
        let nr_wings = line_force_model.nr_wings();

        (0..nr_wings).map(
            |wing_index| {
                self.set_points[wing_index].get_local_wing_angle(&inputs[wing_index], time_step)
            }
        ).collect()
    }

    /// Returns the new internal state for each wing
    pub fn get_internal_states(
        &self,
        time_step: Float,
        inputs: &[ControllerInput],
        line_force_model: &LineForceModel,
    ) -> Vec<Float> {
        let nr_wings = line_force_model.nr_wings();

        (0..nr_wings).map(
            |wing_index| {
                self.set_points[wing_index].get_internal_state(wing_index, line_force_model, &inputs[wing_index], time_step)
            }
        ).collect()
    }
    
    /// Updates the controller. Returns the new output if it is time to update, and `None` otherwise.
    ///
    /// If the controller has input filters, the inputs are added to the filters at every call, and
    /// the output is computed from the filtered inputs.
    pub fn update(
        &mut self,
        time: Float,
        time_step: Float,
        inputs: &[ControllerInput],
        line_force_model: &LineForceModel,
    ) -> Option<ControllerOutput> {
        let filtered_inputs: Option<Vec<ControllerInput>> = self.input_filters.as_mut().map(
            |filters| filters.iter_mut()
                .zip(inputs)
                .map(|(filter, input)| filter.filter(input))
                .collect()
        );

        let inputs = filtered_inputs.as_deref().unwrap_or(inputs);
        let initialization_done = time >= self.start_time;
        let time_to_update = self.time_step_index % self.time_steps_between_updates == 0;
        let first_time_step = self.time_step_index == 0;
        
        // Increment the counter
        self.time_step_index += 1;
        
        if first_time_step || (time_to_update && initialization_done) {
            let local_wing_angles = self.get_local_wing_angles(time_step, inputs, line_force_model);
            let internal_states = self.get_internal_states(time_step, inputs, line_force_model);
            
            return Some(
                ControllerOutput {
                    local_wing_angles,
                    internal_states
                }
            )
        }

        None
    }
}
