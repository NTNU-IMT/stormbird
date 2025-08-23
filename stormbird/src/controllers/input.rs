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

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
/// Structure containing input values that is used by the controllers to set the local wing angles
/// and the section models' internal state. Each member variable contains vectors with data. The
/// length of each vector should equal the number of wings in the simulation
pub struct ControllerInput {
    pub loading: Float,
    pub current_local_wing_angles: Vec<Float>,
    pub current_section_models_internal_state: Vec<Float>,
    pub angles_of_attack: Vec<Float>,
    pub velocity: Vec<Float>,
    pub apparent_wind_directions: Vec<Float>,
}

impl ControllerInput {
    pub fn new(
        loading: Float,
        line_force_model: &LineForceModel,
        simulation_result: &SimulationResult,
        measurement_settings: &FlowMeasurementSettings,
        wind_environment: &WindEnvironment,
    ) -> Self {
        Self {
            loading: loading,
            current_local_wing_angles: line_force_model.local_wing_angles.clone(),
            current_section_models_internal_state: line_force_model.section_models_internal_state(),
            angles_of_attack: measure_angles_of_attack(simulation_result, &measurement_settings.angle_of_attack),
            velocity: measure_wind_velocity_magnitude(simulation_result, &measurement_settings.wind_velocity),
            apparent_wind_directions: measure_apparent_wind_direction(
                simulation_result, &measurement_settings.wind_direction, wind_environment
            ),
        }
    }
}