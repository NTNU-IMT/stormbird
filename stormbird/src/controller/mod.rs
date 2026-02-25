// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementations of a controller for wind propulsion devices intended to be used together
//! with the simulation models in the library. 

pub mod builder;
pub mod input;
pub mod output;
pub mod measurements;
pub mod set_points;
pub mod prelude;

use input::ControllerInput;
use output::ControllerOutput;
use set_points::ControllerSetPoints;
use measurements::FlowMeasurementSettings;

use stormath::type_aliases::Float;

#[derive(Debug, Clone)]
pub struct Controller {
    /// Vector containing the set points for all the sails
    pub set_points: Vec<ControllerSetPoints>,
    /// Structure defining how to measure the representative flow conditions on the sail
    pub flow_measurement_settings: FlowMeasurementSettings,
    /// How often to update the controller
    pub time_steps_between_updates: usize,
    /// When to start using the controller
    pub start_time: Float,
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
        input: &[ControllerInput],
    ) -> Option<Vec<ControllerOutput>> {
        let initialization_done = time >= self.start_time;
        let time_to_update =  self.time_step_index % self.time_steps_between_updates == 0;
        let first_time_step = self.time_step_index == 1;
        
        if first_time_step || (time_to_update && initialization_done) {
            let nr_wings = self.set_points.len();
            
            let mut out = Vec::with_capacity(nr_wings);
            
            for i in 0..nr_wings {
                let output_single = self.set_points[i].get_new_output(&input[i], time_step);
                
                out.push(output_single)
            }
            
            return Some(out)
        }

        None
    }
}
