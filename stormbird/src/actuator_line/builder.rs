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

use super::corrections::{
    lifting_line::LiftingLineCorrectionBuilder,
    empirical_circulation::EmpiricalCirculationCorrection,
};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the actuator line model.
pub struct ActuatorLineBuilder {
    pub line_force_model: LineForceModelBuilder,
    #[serde(default)]
    pub projection_settings: ProjectionSettings,
    #[serde(default)]
    pub solver_settings: SolverSettings,
    #[serde(default)]
    pub sampling_settings: SamplingSettings,
    #[serde(default="ActuatorLineBuilder::default_write_iterations_full_result")]
    pub write_iterations_full_result: usize,
    #[serde(default)]
    pub start_time: Float,
    #[serde(default)]
    pub controller: Option<ControllerBuilder>,
    #[serde(default)]
    pub lifting_line_correction: Option<LiftingLineCorrectionBuilder>,
    #[serde(default)]
    pub empirical_circulation_correction: Option<EmpiricalCirculationCorrection>,
}

impl ActuatorLineBuilder {
    pub fn default_write_iterations_full_result() -> usize {100}

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
            empirical_circulation_correction: None,
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
            sectional_lift_forces_to_project: vec![SpatialVector::default(); nr_span_lines],
            sectional_drag_forces_to_project: vec![SpatialVector::default(); nr_span_lines],
            lifting_line_correction,
            empirical_circulation_correction: self.empirical_circulation_correction.clone(),
        }
    }
}
