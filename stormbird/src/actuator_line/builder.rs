// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use crate::line_force_model::builder::LineForceModelBuilder;
use crate::controller::builder::ControllerBuilder;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use super::projection::ProjectionSettings;
use super::sampling::SamplingSettings;
use super::solver::SolverSettings;
use super::ActuatorLine;

use super::spanflow_damper::SpanflowDamper;

use super::corrections::{
    lifting_line::LiftingLineCorrectionBuilder,
    tip_loss::TipLossCorrection,
};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the actuator line model.
pub struct ActuatorLineBuilder {
    /// Builder for the line force model
    pub line_force_model: LineForceModelBuilder,
    /// Settings for the projection of the forces onto the CFD grid
    #[serde(default)]
    pub projection_settings: ProjectionSettings,
    /// Settings for the solver
    #[serde(default)]
    pub solver_settings: SolverSettings,
    /// Settings for the sampling of the velocity from the CFD grid
    #[serde(default)]
    pub sampling_settings: SamplingSettings,
    /// The number of iterations between each time a full simulation result is written to file
    #[serde(default="ActuatorLineBuilder::default_write_iterations_full_result")]
    pub write_iterations_full_result: usize,
    /// The time to start solving
    #[serde(default)]
    pub start_time: Float,
    /// Optional builder for a controller
    #[serde(default)]
    pub controller: Option<ControllerBuilder>,
    /// Optional correction of the velocity, based on a lifting line model
    #[serde(default)]
    pub lifting_line_correction: Option<LiftingLineCorrectionBuilder>,
    /// Optional empirical tip loss correction of the circulation
    #[serde(default)]
    pub tip_loss_correction: Option<TipLossCorrection>,
    /// Optional damping of the flow along the span
    #[serde(default)]
    pub spanflow_damper: Option<SpanflowDamper>
}

impl ActuatorLineBuilder {
    /// Default number of iterations between each time a full result is written (100)
    pub fn default_write_iterations_full_result() -> usize {100}
    /// Default lifting line correction, which uses the default settings
    pub fn default_lifting_line_correction() -> Option<LiftingLineCorrectionBuilder> {
        Some(
            LiftingLineCorrectionBuilder::default()
        )
    }

    /// Creates a builder with default settings for the given line force model
    pub fn new(line_force_model: LineForceModelBuilder) -> Self {
        Self {
            line_force_model,
            projection_settings: ProjectionSettings::default(),
            solver_settings: SolverSettings::default(),
            sampling_settings: SamplingSettings::default(),
            controller: None,
            write_iterations_full_result: Self::default_write_iterations_full_result(),
            start_time: 0.0,
            lifting_line_correction: None,
            tip_loss_correction: None,
            spanflow_damper: None,
        }
    }

    /// Constructs an actuator line model from the builder data.
    pub fn build(&self) -> ActuatorLine {
        let line_force_model = self.line_force_model.build();

        let nr_span_lines = line_force_model.nr_span_lines();

        if line_force_model.has_dynamic_stall() && self.solver_settings.damping_factor < 1.0 {
            eprintln!(
                "Warning: the actuator line model has a dynamic stall model, but the damping \
                factor of the circulation is {}. The damping acts as an additional time lag of \
                about {:.1} time steps, which also affects the dynamic response of the forces.",
                self.solver_settings.damping_factor,
                1.0 / self.solver_settings.damping_factor
            );
        }

        let controller = self.controller.as_ref().map(|controller_builder| controller_builder
                    .build(&line_force_model)
                    .expect("Could not build the controller"));

        let lifting_line_correction = if let Some(lifting_line_correction_builder) = &self.lifting_line_correction {
            let viscous_core_length_factor = 0.5 * (
                self.projection_settings.projection_function.chord_factor +
                self.projection_settings.projection_function.thickness_factor
            );

            Some(
                lifting_line_correction_builder.build(viscous_core_length_factor, &line_force_model)
            )
        } else {
            None
        };

        ActuatorLine{
            line_force_model,
            projection_settings: self.projection_settings.clone(),
            solver_settings: self.solver_settings.clone(),
            sampling_settings: self.sampling_settings.clone(),
            controller,
            start_time: self.start_time,
            current_iteration: 0,
            write_iterations_full_result: self.write_iterations_full_result,
            ctrl_points_velocity: vec![SpatialVector::default(); nr_span_lines],
            simulation_result: None,
            projected_circulation_strength: vec![0.0; nr_span_lines],
            sectional_lift_forces_to_project: vec![SpatialVector::default(); nr_span_lines],
            sectional_drag_forces_to_project: vec![SpatialVector::default(); nr_span_lines],
            lifting_line_correction,
            tip_loss_correction: self.tip_loss_correction.clone(),
            spanflow_damper: self.spanflow_damper.clone()
        }
    }
}
