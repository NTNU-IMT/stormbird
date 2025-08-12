use serde::{Deserialize, Serialize};

use crate::wind::environment::WindEnvironment;
use crate::line_force_model::LineForceModel;
use crate::common_utils::results::simulation::SimulationResult;

use super::measurements::FlowMeasurementSettings;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
/// Structure containing input values that is used by the controllers to set the local wing angles
/// and the section models' internal state. Each member variable contains vectors with data. The
/// length of each vector should equal the number of wings in the simulation
pub struct ControllerInput {
    pub local_wing_angles: Vec<f64>,
    pub section_models_internal_state: Vec<f64>,
    pub angles_of_attack: Vec<f64>,
    pub velocity: Vec<f64>,
    pub apparent_wind_directions: Option<Vec<f64>>,
}

impl ControllerInput {
    pub fn new(
        line_force_model: &LineForceModel,
        simulation_result: &SimulationResult,
        measurement_settings: &FlowMeasurementSettings,
        wind_environment: Option<&WindEnvironment>,
    ) -> Self {
        todo!()
    }
}