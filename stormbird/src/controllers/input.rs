use serde::{Deserialize, Serialize};

use crate::{
    wind::environment::WindEnvironment,
    line_force_model::LineForceModel,
    common_utils::results::simulation::SimulationResult,
    common_utils::forces_and_moments::CoordinateSystem,
};

use super::measurements::{
    FlowMeasurementSettings,
    measure_angles_of_attack,
    measure_wind_velocity_magnitude,
    measure_apparent_wind_direction,
    measure_float_values
};

use stormath::{spatial_vector::SpatialVector, type_aliases::Float};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
/// Structure containing input values that is used by the controllers to set the local wing angles
/// and the section models' internal state. Each member variable contains vectors with data. The
/// length of each vector should equal the number of wings in the simulation
pub struct ControllerInput {
    /// How much of the max value that should be used
    pub loading: Float,
    /// The current local wing angles, so that the controller know what to change from
    pub current_local_wing_angles: Vec<Float>,
    /// Current internal state
    pub current_section_models_internal_state: Vec<Float>,
    /// Measured angles of attack according to the measurement settings
    pub angles_of_attack: Vec<Float>,
    /// Measured velocity magnitude
    pub velocity: Vec<Float>,
    /// Measured apparent wind direction
    pub apparent_wind_directions: Vec<Float>,
}

impl ControllerInput {
    /// Method that creates input to a controller based on a simulation results structure. This will
    /// then contain information about the lift-induced velocities in the measurements, which might 
    /// be critical for certain controller logics (e.g., effective angle of attack controller). 
    pub fn new_from_simulation_result(
        loading: Float,
        line_force_model: &LineForceModel,
        simulation_result: &SimulationResult,
        measurement_settings: &FlowMeasurementSettings,
        wind_environment: &WindEnvironment,
        use_input_velocity_for_apparent_wind_direction: bool,
    ) -> Self {
        Self {
            loading: loading,
            current_local_wing_angles: line_force_model.local_wing_angles.clone(),
            current_section_models_internal_state: line_force_model.section_models_internal_state(),
            angles_of_attack: measure_angles_of_attack(
                simulation_result, 
                &measurement_settings.angle_of_attack
            ),
            velocity: measure_wind_velocity_magnitude(
                simulation_result, 
                &measurement_settings.wind_velocity
            ),
            apparent_wind_directions: measure_apparent_wind_direction(
                simulation_result, 
                &measurement_settings.wind_direction, 
                wind_environment, 
                use_input_velocity_for_apparent_wind_direction
            ),
        }
    }

    /// Method for creating controller input based on a supplied velocity vector. The intended use 
    /// case is mostly to be able to create controller input based only on the freestream conditions,
    /// which then do not include induced velocities.
    pub fn new_from_velocity(
        loading: Float,
        line_force_model: &LineForceModel,
        velocity: &[SpatialVector],
        measurement_settings: &FlowMeasurementSettings,
        wind_environment: &WindEnvironment,
    ) -> Self {
        let velocity_magnitude: Vec<Float> = velocity.iter().map(|v| v.length()).collect();
        let angles_of_attack = line_force_model.angles_of_attack(
            velocity, CoordinateSystem::Global
        );
        let wind_directions = wind_environment.apparent_wind_directions_from_velocity(velocity);

        let wing_indices = line_force_model.wing_indices.clone();

        Self {
            loading,
            current_local_wing_angles: line_force_model.local_wing_angles.clone(),
            current_section_models_internal_state: line_force_model.section_models_internal_state(),
            angles_of_attack: measure_float_values(
                &angles_of_attack, 
                wing_indices.clone(), 
                &measurement_settings.angle_of_attack
            ),
            velocity: measure_float_values(
                &velocity_magnitude, 
                wing_indices.clone(), 
                &measurement_settings.wind_velocity
            ),
            apparent_wind_directions: measure_float_values(
                &wind_directions, 
                wing_indices.clone(), 
                &measurement_settings.wind_direction
            )
        }
    }
}