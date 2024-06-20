// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of actuator line functionality. 

pub mod projection;

use std::path::Path;
use std::fs;

use serde::{Serialize, Deserialize};

use crate::math_utils::smoothing::gaussian_kernel;

use crate::vec3::Vec3;
use crate::line_force_model::LineForceModel;
use crate::line_force_model::builder::LineForceModelBuilder;
use crate::io_structs::prelude::*;

use projection::Projection;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActuatorLineSettings {
    #[serde(default)]
    pub strength_damping: f64,
    #[serde(default)]
    pub remove_span_velocity: bool,
    #[serde(default)]
    pub velocity_aligned_projection: bool,
}

impl Default for ActuatorLineSettings {
    fn default() -> Self {
        Self {
            strength_damping: 0.0,
            remove_span_velocity: false,
            velocity_aligned_projection: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the actuator line model.
pub struct ActuatorLineBuilder {
    pub line_force_model_builder: LineForceModelBuilder,
    #[serde(default)]
    pub projection: Projection,
    #[serde(default)]
    pub settings: ActuatorLineSettings,
}

impl ActuatorLineBuilder {
    pub fn new(line_force_model_builder: LineForceModelBuilder) -> Self {
        Self {
            line_force_model_builder,
            projection: Projection::default(),
            settings: ActuatorLineSettings::default(),
        }
    }

    /// Constructs a actuator line model from the builder data.
    pub fn build(&self) -> ActuatorLine {
        let line_force_model = self.line_force_model_builder.build();

        let nr_span_lines = line_force_model.nr_span_lines();

        ActuatorLine{
            line_force_model,
            projection: self.projection.clone(),
            ctrl_points_velocity: vec![Vec3::default(); nr_span_lines],
            results: Vec::new(),
            settings: self.settings.clone(),
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
    pub ctrl_points_velocity: Vec<Vec3>,
    /// Results from the model
    pub results: Vec<SimulationResult>,
    /// Numerical settings
    pub settings: ActuatorLineSettings,
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

    pub fn get_weighted_velocity_integral_terms_for_cell(
        &self, 
        line_index: usize, 
        velocity: Vec3, 
        cell_center: Vec3, 
        cell_volume: f64
    ) -> (Vec3, f64) {
        let span_line = self.line_force_model.span_lines()[line_index];
        let chord_vector = self.line_force_model.chord_vectors()[line_index];

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

    pub fn calculate_and_add_result(&mut self) {        
        let mut ctrl_points_velocity = self.ctrl_points_velocity.clone();
         
        if self.settings.remove_span_velocity {
            ctrl_points_velocity = self.line_force_model.remove_span_velocity(
                &ctrl_points_velocity
            );
        }

        let result = self.calculate_result(&ctrl_points_velocity);

        self.results.push(result);
    }

    /// Takes the estimated velocity on at the control points as input and calculates a simulation
    /// result from the line force model.
    pub fn calculate_result(&mut self, velocity_ctrl_points: &[Vec3]) -> SimulationResult {
        let ctrl_points = self.line_force_model.ctrl_points();

        let new_estimated_circulation_strength = self.line_force_model.circulation_strength(
            &velocity_ctrl_points
        );

        let circulation_strength = if self.settings.strength_damping > 0.0 {
            let previous_strength = if self.results.len() > 0 {
                self.results.last().unwrap().force_input.circulation_strength.clone()
            } else {
                vec![0.0; self.line_force_model.nr_span_lines()]
            };

            new_estimated_circulation_strength.iter().zip(previous_strength.iter()).map(|(new, old)| {
                old + (1.0 - self.settings.strength_damping) * (new - old)
            }).collect()
        } else {
            new_estimated_circulation_strength
        };

        let angles_of_attack = self.line_force_model.angles_of_attack(&velocity_ctrl_points);

        // TODO: This must be undated to handle moving wings!
        let force_input = SectionalForcesInput {
            circulation_strength,
            velocity: velocity_ctrl_points.to_vec(),
            angles_of_attack,
            acceleration: vec![Vec3::default(); self.line_force_model.nr_span_lines()],
            angles_of_attack_derivative: vec![0.0; self.line_force_model.nr_span_lines()],
            rotation_velocity: Vec3::default(),
        };

        let sectional_forces = self.line_force_model.sectional_forces(&force_input);

        let integrated_forces = sectional_forces.integrate_forces(&self.line_force_model);
        let integrated_moments = sectional_forces.integrate_moments(&self.line_force_model);

        SimulationResult {
            ctrl_points,
            force_input,
            sectional_forces,
            integrated_forces,
            integrated_moments,
        }
    }

    /// Writes the resulting values from the line force model to a file. 
    pub fn write_results(&self) {
        let serialized_result = serde_json::to_string(&self.results).unwrap();

        let file_path = "actuator_line_results.json".to_owned();

        fs::write(file_path, serialized_result).expect("Unable to write result file");
    }

    /// Computes a distributed body force at a given point in space.
    pub fn distributed_body_force_at_point(&self, point: Vec3) -> Vec3 {
        let span_lines = self.line_force_model.span_lines();
        let chord_vectors = self.line_force_model.chord_vectors();
        
        let result = self.results.last().unwrap();

        let sectional_forces_to_project = self.line_force_model.sectional_circulatory_forces(
            &result.force_input.circulation_strength, &result.force_input.velocity
        );
        
        let mut body_force = Vec3::default();

        for i in 0..self.line_force_model.nr_span_lines() {
            let effective_chord_vector = if self.settings.velocity_aligned_projection {
                result.force_input.velocity[i].normalize() * chord_vectors[i].length()
            } else {
                chord_vectors[i]
            };

            let projection = self.projection.projection_value_at_point(
                point, effective_chord_vector, &span_lines[i]
            );

            body_force += sectional_forces_to_project[i] * projection;
        }

        body_force
    }

    pub fn distributed_body_force_weight_at_point(&self, point: Vec3) -> f64 {
        let mut body_force_weight = 0.0;

        let span_lines = self.line_force_model.span_lines();
        let chord_vectors = self.line_force_model.chord_vectors();

        let result = self.results.last().unwrap();

        for i in 0..self.line_force_model.nr_span_lines() {
            let effective_chord_vector = if self.settings.velocity_aligned_projection {
                result.force_input.velocity[i].normalize() * chord_vectors[i].length()
            } else {
                chord_vectors[i]
            };

            body_force_weight += self.projection.projection_value_at_point(
                point, effective_chord_vector, &span_lines[i]
            );
        }

        body_force_weight
    }
}