use crate::line_force_model::builder::LineForceModelBuilder;
use crate::controllers::ControllerBuilder;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;

use super::projection::Projection;
use super::SolverSettings;
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
    pub controller: Option<ControllerBuilder>,
}

impl ActuatorLineBuilder {
    pub fn new(line_force_model: LineForceModelBuilder) -> Self {
        Self {
            line_force_model,
            projection: Projection::default(),
            solver_settings: SolverSettings::default(),
            controller: None,
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
            controller,
        }
    }
}

