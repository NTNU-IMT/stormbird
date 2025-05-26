use crate::line_force_model::builder::LineForceModelBuilder;
use crate::controllers::ControllerBuilder;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;

use super::projection::Projection;
use super::settings::*;
use super::ActuatorLine;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the actuator line model.
pub struct ActuatorLineBuilder {
    pub line_force_model: LineForceModelBuilder,
    #[serde(default)]
    pub projection: Projection,
    #[serde(default)]
    pub solver_settings: SolverSettings,
    #[serde(default)]
    pub sampling_settings: SamplingSettings,
    #[serde(default)]
    pub controller: Option<ControllerBuilder>,
    #[serde(default="ActuatorLineBuilder::default_write_iterations_full_result")]
    pub write_iterations_full_result: usize,
    #[serde(default)]
    pub start_iteration: usize,
    #[serde(default)]
    pub extrapolate_end_velocities: bool,
}

impl ActuatorLineBuilder {
    pub fn default_write_iterations_full_result() -> usize {500}

    pub fn new(line_force_model: LineForceModelBuilder) -> Self {
        Self {
            line_force_model,
            projection: Projection::default(),
            solver_settings: SolverSettings::default(),
            sampling_settings: SamplingSettings::default(),
            controller: None,
            write_iterations_full_result: Self::default_write_iterations_full_result(),
            start_iteration: 0,
            extrapolate_end_velocities: false,
        }
    }

    /// Constructs a actuator line model from the builder data.
    pub fn build(&self) -> ActuatorLine {
        let line_force_model = self.line_force_model.build();

        let nr_span_lines = line_force_model.nr_span_lines();

        let controller = if let Some(controller_builder) = &self.controller {
            Some(controller_builder.build())
        } else {
            None
        };

        ActuatorLine{
            line_force_model,
            projection: self.projection.clone(),
            ctrl_points_velocity: vec![SpatialVector::<3>::default(); nr_span_lines],
            simulation_result: None,
            solver_settings: self.solver_settings.clone(),
            sampling_settings: self.sampling_settings.clone(),
            controller,
            start_iteration: self.start_iteration,
            current_iteration: 0,
            write_iterations_full_result: self.write_iterations_full_result,
            extrapolate_end_velocities: self.extrapolate_end_velocities,
        }
    }
}

