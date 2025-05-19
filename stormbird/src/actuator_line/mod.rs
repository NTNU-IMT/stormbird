// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of actuator line functionality. 

use std::fs;
use std::path::Path;

pub mod projection;
pub mod builder;

use serde::{Serialize, Deserialize};

use stormath::smoothing::gaussian::gaussian_kernel;

use stormath::spatial_vector::SpatialVector;
use crate::line_force_model::LineForceModel;

use crate::common_utils::prelude::*;

use crate::controllers::Controller;
use crate::io_utils::csv_data;

use projection::Projection;
use builder::ActuatorLineBuilder;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverSettings {
    #[serde(default)]
    pub strength_damping: f64,
}

#[derive(Debug, Clone)]
/// Structure for representing an actuator line model. 
pub struct ActuatorLine {
    /// The line force model used to compute forces on each line segment as a function of the local
    /// velocity.
    pub line_force_model: LineForceModel,
    /// Enum, with an internal structure, that determines how forces are projected in a CFD 
    /// simulation
    pub projection: Projection,
    /// Vector to store interpolated velocity values for each control point
    pub ctrl_points_velocity: Vec<SpatialVector<3>>,
    /// Results from the model
    pub simulation_result: Option<SimulationResult>,
    /// Numerical settings
    pub solver_settings: SolverSettings,
    /// Dynamic optimizer that can be optionally used to optimize the settings in the model
    pub controller: Option<Controller>,
}

impl ActuatorLine {
    /// Loads a file containing settings for an actuator line builder, and constructs a new model 
    /// based on this data.
    pub fn new_from_file<P: AsRef<Path>>(file_path: P) -> Self {
        let read_file_result = fs::read_to_string(file_path).unwrap();

        Self::new_from_string(&read_file_result)
    }

    /// Constructs a new model from a string containing settings for an actuator line builder.
    pub fn new_from_string(builder_string: &str) -> Self {
        let builder: ActuatorLineBuilder = serde_json::from_str(builder_string).unwrap();

        builder.build()
    }

    /// Function used to query the actuator line model for the weighted velocity integral term for
    /// a given cell. This is used to compute the body force in the CFD simulation.
    pub fn get_weighted_velocity_integral_terms_for_cell(
        &self, 
        line_index: usize, 
        velocity: SpatialVector<3>, 
        cell_center: SpatialVector<3>, 
        cell_volume: f64
    ) -> (SpatialVector<3>, f64) {
        let span_line = self.line_force_model.span_line_at_index(line_index);
        let chord_vector = self.line_force_model.global_chord_vector_at_index(line_index);

        let projection_value_org = self.projection.projection_value_at_point(
            cell_center, chord_vector, &span_line
        );

        let projection_value = if projection_value_org > 0.0 {
            let line_coordinates = span_line.line_coordinates(cell_center, chord_vector);
            
            // TODO: check if this is necessary
            let span_projection = gaussian_kernel(
                line_coordinates.span, 
                0.0, 
                0.5 * span_line.length()
            );

            projection_value_org * span_projection
        } else {
            0.0
        };

        let denominator = cell_volume * projection_value;
        let numerator = velocity * denominator; 

        (numerator, denominator)
    }

    pub fn do_step(&mut self, time: f64, time_step: f64){
        let solver_result = self.solve(&self.ctrl_points_velocity);

        let simulation_result = self.line_force_model.calculate_simulation_result(
            &solver_result, 
            time, 
            time_step
        );

        //self.line_force_model.update_flow_derivatives(&result);

        self.simulation_result = Some(simulation_result);        
    }

    pub fn update_controller(&mut self, time_step: f64) -> bool {
        let controller_output = if let Some(controller) = &mut self.controller {
            let model_state = self.line_force_model.model_state();

            let simulation_result = self.simulation_result.as_ref().unwrap();
            
            controller.update(time_step, &model_state, simulation_result)
        } else {
            None
        };

        let mut need_update = false;

        if let Some(controller_output) = controller_output {
            if let Some(new_angles) = controller_output.local_wing_angles {
                self.line_force_model.local_wing_angles = new_angles;
            }
            
            if let Some(new_internal_states) = controller_output.section_models_internal_state {
                self.line_force_model.set_section_models_internal_state(&new_internal_states);
            }

            need_update = true;

            let new_model_state = self.line_force_model.model_state();

            new_model_state.write_to_csv_file("model_state.csv");

        }

        need_update
    }

    /// Takes the estimated velocity on at the control points as input and calculates a simulation
    /// result from the line force model.
    pub fn solve(&self, ctrl_point_velocity: &[SpatialVector<3>]) -> SolverResult {
        let new_estimated_circulation_strength = self.line_force_model.circulation_strength(
            &ctrl_point_velocity, CoordinateSystem::Global
        );

        let circulation_strength = if self.solver_settings.strength_damping > 0.0 {

            let previous_strength = if let Some(simulation_result) = &self.simulation_result {
                simulation_result.force_input.circulation_strength.clone()
            } else {
                vec![0.0; self.line_force_model.nr_span_lines()]
            };

            new_estimated_circulation_strength.iter().zip(previous_strength.iter()).map(|(new, old)| {
                old + (1.0 - self.solver_settings.strength_damping) * (new - old)
            }).collect()
        } else {
            new_estimated_circulation_strength
        };

        let residual = self.line_force_model.average_residual_absolute(
            &circulation_strength, 
            ctrl_point_velocity,
            CoordinateSystem::Global
        );

        SolverResult {
            circulation_strength,
            ctrl_point_velocity: ctrl_point_velocity.to_vec(),
            iterations: 1,
            residual,
        }
    }

    /// Writes the resulting values from the line force model to a file. 
    pub fn write_results(&self) {
        if let Some(simulation_result) = &self.simulation_result {
            let (header, data) = simulation_result.as_reduced_flatten_csv_string();

            let _ = csv_data::create_or_append_header_and_data_strings_file(
                "actuator_line_results.csv", 
                &header, 
                &data
            );
        }
    }

    /// Computes a distributed body force at a given point in space.
    pub fn distributed_body_force_at_point(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        let projection_weights = self.line_segments_projection_weights_at_point(point);

        if let Some(simulation_result) = &self.simulation_result {
            let sectional_forces_to_project = self.line_force_model.
                sectional_circulatory_forces(
                    &simulation_result.force_input.circulation_strength, 
                    &simulation_result.force_input.velocity
                );
            
            let mut body_force = SpatialVector::<3>::default();

            for i in 0..self.line_force_model.nr_span_lines() {
                body_force += sectional_forces_to_project[i] * projection_weights[i];
            }

            body_force
        } else {
            SpatialVector::<3>::default()
        }        
    }

    /// Computes the body force weights for each line element at a given point in space.
    pub fn line_segments_projection_weights_at_point(&self, point: SpatialVector<3>) -> Vec<f64> {
        let span_lines = self.line_force_model.span_lines();
        let chord_vectors = self.line_force_model.global_chord_vectors();

        let mut projection_values = Vec::with_capacity(self.line_force_model.nr_span_lines());

        for i in 0..self.line_force_model.nr_span_lines() {
            projection_values.push(self.projection.projection_value_at_point(
                point, chord_vectors[i], &span_lines[i]
            ));
        }

        projection_values
    }

    /// Computes the sum of the projection weights for all line elements at a given point in space.
    pub fn summed_projection_weights_at_point(&self, point: SpatialVector<3>) -> f64 {
        self.line_segments_projection_weights_at_point(point).iter().sum()
    }

    /// Checks which line element is dominating at a given point in space by comparing the 
    /// projection weights of each line element.
    pub fn dominating_line_element_index_at_point(&self, point: SpatialVector<3>) -> usize {
        let projection_weights = self.line_segments_projection_weights_at_point(point);

        let mut max_weight = -1.0;
        let mut max_index = 0;

        for (i, weight) in projection_weights.iter().enumerate() {
            if *weight > max_weight {
                max_weight = *weight;
                max_index = i;
            }
        }

        max_index
    }
}