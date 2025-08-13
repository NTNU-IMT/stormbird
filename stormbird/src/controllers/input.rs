use serde::{Deserialize, Serialize};

use crate::{
    wind::environment::WindEnvironment,
    line_force_model::LineForceModel,
    common_utils::results::simulation::SimulationResult
};

use super::measurements::{
    FlowMeasurementSettings,
    measure_angles_of_attack,
    measure_wind_velocity_magnitude,
    measure_apparent_wind_direction
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Structure containing input values that is used by the controllers to set the local wing angles
/// and the section models' internal state. Each member variable contains vectors with data. The
/// length of each vector should equal the number of wings in the simulation
pub struct ControllerInput {
    pub local_wing_angles: Vec<f64>,
    pub section_models_internal_state: Vec<f64>,
    pub angles_of_attack: Vec<f64>,
    pub velocity: Vec<f64>,
    pub apparent_wind_directions: Vec<f64>,
}

impl ControllerInput {
    pub fn new(
        line_force_model: &LineForceModel,
        simulation_result: &SimulationResult,
        measurement_settings: &FlowMeasurementSettings,
        wind_environment: &WindEnvironment,
    ) -> Self {
        Self {
            local_wing_angles: line_force_model.local_wing_angles.clone(),
            section_models_internal_state: line_force_model.section_models_internal_state(),
            angles_of_attack: measure_angles_of_attack(simulation_result, &measurement_settings.angle_of_attack),
            velocity: measure_wind_velocity_magnitude(simulation_result, &measurement_settings.wind_velocity),
            apparent_wind_directions: measure_apparent_wind_direction(
                simulation_result, &measurement_settings.wind_direction, wind_environment
            ),
        }
    }
}