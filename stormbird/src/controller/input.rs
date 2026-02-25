// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

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
    pub current_local_wing_angle: Float,
    /// Current internal state
    pub current_section_model_internal_state: Float,
    /// Measured angles of attack according to the measurement settings
    pub angle_of_attack: Float,
    /// Measured velocity magnitude
    pub velocity: Float,
    /// Measured apparent wind direction
    pub apparent_wind_direction: Float,
}

impl ControllerInput {
    /// Method that creates input to a controller based on a simulation results structure. This will
    /// then contain information about the lift-induced velocities in the measurements, which might 
    /// be critical for certain controller logic (e.g., effective angle of attack controller). 
    pub fn new_from_simulation_result(
        loading: Float,
        line_force_model: &LineForceModel,
        simulation_result: &SimulationResult,
        measurement_settings: &FlowMeasurementSettings,
        wind_environment: &WindEnvironment,
        use_input_velocity_for_apparent_wind_direction: bool,
    ) -> Vec<Self> {
        let nr_wings = line_force_model.nr_wings();
        
        let section_models_internal_state = line_force_model.section_models_internal_state();
        
        let angles_of_attack = measure_angles_of_attack(
            simulation_result, 
            &measurement_settings.angle_of_attack
        );
        
        let velocities = measure_wind_velocity_magnitude(
            simulation_result, 
            &measurement_settings.wind_velocity
        );
        
        let apparent_wind_directions = measure_apparent_wind_direction(
            simulation_result, 
            &measurement_settings.wind_direction, 
            wind_environment, 
            line_force_model,
            use_input_velocity_for_apparent_wind_direction
        );
        
        let mut out: Vec<Self> = Vec::with_capacity(nr_wings);
        
        for i in 0..nr_wings {
            out.push(
                Self {
                    loading: loading,
                    current_local_wing_angle: line_force_model.local_wing_angles[i],
                    current_section_model_internal_state: section_models_internal_state[i],
                    angle_of_attack: angles_of_attack[i],
                    velocity: velocities[i],
                    apparent_wind_direction: apparent_wind_directions[i]
                }
            )
        }
        
        out
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
    ) -> Vec<Self> {
        let nr_wings = line_force_model.nr_wings();
        let wing_indices = line_force_model.wing_indices.clone();
        
        let velocities_all_sections: Vec<Float> = velocity.iter().map(|v| v.length()).collect();
        let angles_of_attack_all_sections = line_force_model.angles_of_attack(
            velocity, CoordinateSystem::Global
        );
        
        let angles_of_attack = measure_float_values(
            &angles_of_attack_all_sections, 
            wing_indices.clone(), 
            &measurement_settings.angle_of_attack
        );
        
        let velocities = measure_float_values(
            &velocities_all_sections, 
            wing_indices.clone(), 
            &measurement_settings.wind_velocity
        );
        
        let wind_directions = wind_environment.apparent_wind_direction_from_velocity_and_line_force_model(
            velocity,
            line_force_model
        );
        
        let apparent_wind_directions = measure_float_values(
            &wind_directions, 
            wing_indices.clone(), 
            &measurement_settings.wind_direction
        );
        
        let section_models_internal_state = line_force_model.section_models_internal_state();


        let mut out: Vec<Self> = Vec::with_capacity(nr_wings);
        
        for i in 0..nr_wings {
            out.push(
                Self {
                    loading: loading,
                    current_local_wing_angle: line_force_model.local_wing_angles[i],
                    current_section_model_internal_state: section_models_internal_state[i],
                    angle_of_attack: angles_of_attack[i],
                    velocity: velocities[i],
                    apparent_wind_direction: apparent_wind_directions[i]
                }
            )
        }
        
        out
    }
}
