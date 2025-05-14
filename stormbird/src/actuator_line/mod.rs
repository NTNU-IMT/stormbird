// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of actuator line functionality. 

pub mod projection;

use std::path::Path;
use std::fs;

use serde::{Serialize, Deserialize};

use stormath::smoothing::gaussian::gaussian_kernel;

use stormath::spatial_vector::SpatialVector;
use crate::line_force_model::LineForceModel;
use crate::line_force_model::builder::LineForceModelBuilder;
use crate::common_utils::prelude::*;

use crate::controllers::dynamic_optimizer::DynamicOptimizer;

use projection::Projection;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverSettings {
    #[serde(default)]
    pub strength_damping: f64,
}

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
    pub optimizer: Option<DynamicOptimizer>,
}

impl ActuatorLineBuilder {
    pub fn new(line_force_model: LineForceModelBuilder) -> Self {
        Self {
            line_force_model,
            projection: Projection::default(),
            solver_settings: SolverSettings::default(),
            optimizer: None,
        }
    }

    /// Constructs a actuator line model from the builder data.
    pub fn build(&self) -> ActuatorLine {
        let line_force_model = self.line_force_model.build();

        let nr_span_lines = line_force_model.nr_span_lines();

        ActuatorLine{
            line_force_model,
            projection: self.projection.clone(),
            ctrl_points_velocity: vec![SpatialVector::<3>::default(); nr_span_lines],
            time: Vec::new(),
            results: Vec::new(),
            solver_settings: self.solver_settings.clone(),
            optimizer: self.optimizer.clone(),
        }
    }
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
    /// The time history of the simulation
    pub time: Vec<f64>,
    /// Results from the model
    pub results: Vec<SimulationResult>,
    /// Numerical settings
    pub solver_settings: SolverSettings,
    /// Dynamic optimizer that can be optionally used to optimize the settings in the model
    pub optimizer: Option<DynamicOptimizer>,
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

    pub fn do_step(&mut self, time_step: f64, time: f64) -> bool{        
        let solver_result = self.solve(&self.ctrl_points_velocity);

        let result = self.line_force_model.calculate_simulation_result(&solver_result, time_step);

        //self.line_force_model.update_flow_derivatives(&result);

        self.results.push(result);
        self.time.push(time);

        let new_local_wing_angles = if let Some(optimizer) = &mut self.optimizer {
            optimizer.update(
                &self.time,
                &self.results,
            )
        } else {
            None
        };

        let mut need_update = false;

        if let Some(new_angles) = new_local_wing_angles {
            for i in 0..self.line_force_model.nr_wings() {
                self.line_force_model.local_wing_angles[i] = new_angles[i];
            }

            need_update = true;
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
            let previous_strength = if self.results.len() > 0 {
                self.results.last().unwrap().force_input.circulation_strength.clone()
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
        let serialized_result = serde_json::to_string(&self.results).unwrap();

        let file_path = "actuator_line_results.json".to_owned();

        fs::write(file_path, serialized_result).expect("Unable to write result file");
    }

    /// Computes a distributed body force at a given point in space.
    pub fn distributed_body_force_at_point(&self, point: SpatialVector<3>) -> SpatialVector<3> {
        let projection_weights = self.line_segments_projection_weights_at_point(point);

        let result = self.results.last().unwrap();

        let sectional_forces_to_project = self.line_force_model.sectional_circulatory_forces(
            &result.force_input.circulation_strength, &result.force_input.velocity
        );
        
        let mut body_force = SpatialVector::<3>::default();

        for i in 0..self.line_force_model.nr_span_lines() {
            body_force += sectional_forces_to_project[i] * projection_weights[i];
        }

        body_force
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