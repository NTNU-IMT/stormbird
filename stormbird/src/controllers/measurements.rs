use std::ops::Range;

use serde::{Deserialize, Serialize};

use stormath::statistics;

use crate::{
    common_utils::results::simulation::SimulationResult,
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

fn measure_float_values(
    values: &[f64],
    wing_indices: Vec<Range<usize>>,
    measurement_settings: &MeasurementSettings,
) -> Vec<f64> {

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
) -> Vec<f64> {
    measure_float_values(
        &simulation_result.force_input.angles_of_attack,
        simulation_result.wing_indices.clone(),
        measurement_settings
    )
}

pub fn measure_wind_velocity_magnitude(
    simulation_result: &SimulationResult,
    measurement_settings: &MeasurementSettings,
) -> Vec<f64> {
    let velocity_magnitude: Vec<f64> = simulation_result.force_input.velocity
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
    wind_environment: &WindEnvironment
) -> Vec<f64> {
    let relevant_velocities = simulation_result.felt_velocity_minus_rotational_motion();

    let wind_directions: Vec<f64> = relevant_velocities.iter()
        .map(
            |velocity| {
                wind_environment.zero_direction_vector.signed_angle_between(
                    *velocity, 
                    wind_environment.up_direction
                )
            }
        ).collect();

    measure_float_values(
        &wind_directions, 
        simulation_result.wing_indices.clone(),
        measurement_settings
    )
}