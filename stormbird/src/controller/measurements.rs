// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use std::ops::Range;

use serde::{Deserialize, Serialize};

use stormath::statistics;
use stormath::type_aliases::Float;

use crate::{
    common_utils::results::simulation::SimulationResult, 
    line_force_model::LineForceModel, 
    wind::environment::WindEnvironment,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum MeasurementType {
    #[default]
    Mean,
    Max,
    Min,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
/// Settings used when measuring flow characteristics from a simulation, when the intention is to 
/// generate input to a controller.
pub struct MeasurementSettings {
    #[serde(default)]
    pub measurement_type: MeasurementType,
    #[serde(default)]
    pub start_index: usize,
    #[serde(default)]
    pub end_offset: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowMeasurementSettings {
    #[serde(default)]
    pub angle_of_attack: MeasurementSettings,
    #[serde(default)]
    pub wind_direction: MeasurementSettings,
    #[serde(default)]
    pub wind_velocity: MeasurementSettings,
}

pub fn measure_float_values(
    values: &[Float],
    wing_indices: Vec<Range<usize>>,
    measurement_settings: &MeasurementSettings,
) -> Vec<Float> {

    let nr_wings = wing_indices.len();

    let mut out = vec![0.0; nr_wings];

    for i in 0..nr_wings {
        let wing_values = values[wing_indices[i].clone()].to_vec();
        let nr_strips = wing_values.len();

        let start_index = measurement_settings.start_index;
        let end_index = (nr_strips - measurement_settings.end_offset - 1).max(start_index+1).min(nr_strips-1);

        out[i] = match measurement_settings.measurement_type {
            MeasurementType::Mean => statistics::mean(
                &wing_values[start_index..end_index]
            ),
            MeasurementType::Max => statistics::max(
                &wing_values[start_index..end_index]
            ),
            MeasurementType::Min => statistics::min(
                &wing_values[start_index..end_index]
            )
        };
    }

    out
}

pub fn measure_angles_of_attack(
    simulation_result: &SimulationResult,
    measurement_settings: &MeasurementSettings,
) -> Vec<Float> {
    measure_float_values(
        &simulation_result.force_input.angles_of_attack,
        simulation_result.wing_indices.clone(),
        measurement_settings
    )
}

pub fn measure_wind_velocity_magnitude(
    simulation_result: &SimulationResult,
    measurement_settings: &MeasurementSettings,
) -> Vec<Float> {
    let velocity_magnitude: Vec<Float> = simulation_result.force_input.velocity
        .iter()
        .map(|v| v.length())
        .collect();

    measure_float_values(
        &velocity_magnitude,
        simulation_result.wing_indices.clone(),
        measurement_settings
    )
}

pub fn measure_apparent_wind_direction(
    simulation_result: &SimulationResult,
    measurement_settings: &MeasurementSettings,
    wind_environment: &WindEnvironment,
    line_force_model: &LineForceModel,
    use_input_velocity: bool,
) -> Vec<Float> {
    let relevant_velocities = if use_input_velocity {
        simulation_result.felt_input_velocity_minus_rotational_motion()
    } else {
        simulation_result.felt_velocity_minus_rotational_motion()
    };

    let wind_directions = wind_environment
        .apparent_wind_direction_from_velocity_and_line_force_model(
            &relevant_velocities, 
            line_force_model
        );

    measure_float_values(
        &wind_directions, 
        simulation_result.wing_indices.clone(),
        measurement_settings
    )
}