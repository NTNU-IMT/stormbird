// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of actuator line functionality. 

pub mod projection;
pub mod velocity_sampling;
pub mod settings;

use std::path::Path;
use std::fs;

use serde::{Serialize, Deserialize};

use crate::math_utils::smoothing::gaussian_kernel;

use crate::vec3::Vec3;
use crate::line_force_model::LineForceModel;
use crate::line_force_model::builder::LineForceModelBuilder;
use crate::io_structs::prelude::*;

use projection::Projection;
use velocity_sampling::{VelocitySampling, VelocitySamplingBuilder};
use settings::ActuatorLineSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the actuator line model.
pub struct ActuatorLineBuilder {
    pub force_model: LineForceModelBuilder,
    #[serde(default)]
    pub projection: Projection,
    #[serde(default)]
    pub velocity_sampling: VelocitySamplingBuilder,
    #[serde(default)]
    pub settings: ActuatorLineSettings,
    #[serde(default="ActuatorLineBuilder::default_project_upscale_factor")]
    pub project_upscale_factor: usize,
}

impl ActuatorLineBuilder {
    pub fn default_project_upscale_factor() -> usize { 1 }

    pub fn new(force_model: LineForceModelBuilder) -> Self {
        Self {
            force_model,
            projection: Projection::default(),
            velocity_sampling: VelocitySamplingBuilder::default(),
            settings: ActuatorLineSettings::default(),
            project_upscale_factor: Self::default_project_upscale_factor(),
        }
    }

    /// Constructs a actuator line model from the builder data.
    pub fn build(&self) -> ActuatorLine {
        let force_model_sampling = self.force_model.build();
        let force_model_projection = self.force_model.build_with_nr_sections(self.force_model.nr_sections * self.project_upscale_factor);
        let projection = self.projection.clone();

        let velocity_sampling = self.velocity_sampling.build(force_model_sampling.nr_span_lines());

        ActuatorLine{
            force_model_sampling,
            force_model_projection,
            projection,
            velocity_sampling,
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
    pub force_model_sampling: LineForceModel,
    pub force_model_projection: LineForceModel,
    /// Enum, with an internal structure, that determines how forces are projected in a CFD 
    /// simulation
    pub projection: Projection,
    /// Model that determines how the velocity is sampled around the line segments
    pub velocity_sampling: VelocitySampling,
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

    pub fn clear_cell_information(&mut self) {
        self.velocity_sampling.reset();
    }

    pub fn add_cell_information(&mut self, center: Vec3, velocity: Vec3, volume: f64) {
        let sampling_span_lines = self.force_model_sampling.span_lines();
        let sampling_chord_vectors = self.force_model_sampling.chord_vectors();

        for i in 0..self.force_model_sampling.nr_span_lines() {
            let span_line    = &sampling_span_lines[i];
            let chord_vector = sampling_chord_vectors[i];

            let projection_value_org = self.projection.projection_value_at_point(
                center, chord_vector, span_line
            );

            if projection_value_org > 0.0 {
                let line_coordinates = span_line.line_coordinates(center, chord_vector);

                let span_projection = gaussian_kernel(
                    line_coordinates.span, 
                    0.0, 
                    self.velocity_sampling.span_gaussian_width_factor * span_line.length()
                );

                let projection_value = projection_value_org * span_projection;

                self.velocity_sampling.add_cell_information(i, velocity, volume, projection_value);
            }
        }
    }

    pub fn calculate_and_add_result(&mut self) {        
        let mut ctrl_points_velocity_sampling = self.velocity_sampling.freestream_velocity();
         
        if self.settings.remove_span_velocity {
            ctrl_points_velocity_sampling = self.force_model_sampling.remove_span_velocity(
                &ctrl_points_velocity_sampling
            );
        }

        let result = self.calculate_result(&ctrl_points_velocity_sampling);

        self.results.push(result);
    }

    /// Takes the estimated velocity on at the control points as input and calculates a simulation
    /// result from the line force model.
    pub fn calculate_result(&mut self, velocity_ctrl_points_sampling: &[Vec3]) -> SimulationResult {
        let mut result = SimulationResult::default();

        let span_lines_projection = self.force_model_projection.span_lines();

        result.ctrl_points = span_lines_projection.iter().map(|line| {
            line.ctrl_point()
        }).collect();

        let mapping = 
            self.force_model_sampling.nr_span_lines() != 
            self.force_model_projection.nr_span_lines();

        result.velocity = if mapping {
            self.upscaled_velocity(velocity_ctrl_points_sampling)
        } else {
            velocity_ctrl_points_sampling.to_vec()
        };

        let sampling_strength = self.force_model_sampling.circulation_strength(&velocity_ctrl_points_sampling);
        
        let new_calculated_strength = if mapping {
            self.upscaled_strength(&sampling_strength)
        } else {
            sampling_strength
        };

        if self.settings.strength_damping > 0.0 {
            let previous_strength = if self.results.len() > 0 {
                self.results.last().unwrap().circulation_strength.clone()
            } else {
                vec![0.0; self.force_model_projection.nr_span_lines()]
            };

            for i in 0..new_calculated_strength.len() {
                let change = new_calculated_strength[i] - previous_strength[i];
                
                result.circulation_strength.push(
                    previous_strength[i] + (1.0 - self.settings.strength_damping) * change
                );
            }
        } else {
            result.circulation_strength = new_calculated_strength;
        }

        // TODO: This must be uddated to handle moving wings!
        let force_input = SectionalForcesInput {
            circulation_strength: result.circulation_strength.clone(),
            felt_velocity: result.velocity.clone(),
            acceleration: vec![Vec3::default(); self.force_model_projection.nr_span_lines()],
            chord_rotation_velocity: vec![0.0; self.force_model_projection.nr_span_lines()],
        };

        result.sectional_forces  = self.force_model_projection.sectional_forces(&force_input);

        result.integrated_forces = result.sectional_forces.integrate_forces(&self.force_model_projection);
        result.integrated_moments = result.sectional_forces.integrate_moments(&self.force_model_projection);

        result
    }

    pub fn upscaled_velocity(&self, sampling_velocity: &[Vec3]) -> Vec<Vec3> {
        self.force_model_sampling.map_velocity_gaussian(
            sampling_velocity, 
            &self.force_model_projection,
            self.settings.gaussian_mapping_length_factor
        )
    }

    pub fn upscaled_strength(&self, sampling_strength: &[f64]) -> Vec<f64> {
        self.force_model_sampling.map_strength_gaussian(
            sampling_strength, 
            &self.force_model_projection, 
            self.settings.gaussian_mapping_end_correction, 
            self.settings.gaussian_mapping_length_factor
        )
    }

    /// Writes the resulting values from the line force model to a file. 
    pub fn write_results(&self) {
        let serialized_result = serde_json::to_string(&self.results).unwrap();

        let file_path = "actuator_line_results.json".to_owned();

        fs::write(file_path, serialized_result).expect("Unable to write result file");
    }

    /// Computes a distributed body force at a given point in space.
    pub fn distributed_body_force_at_point(&self, point: Vec3) -> Vec3 {
        let span_lines_projection = self.force_model_projection.span_lines();
        let chord_vectors_projection = self.force_model_projection.chord_vectors();
        let mut body_force = Vec3::default();

        let result = self.results.last().unwrap();

        let sectional_forces_to_project = self.force_model_projection.sectional_circulatory_forces(&result.circulation_strength, &result.velocity);
        
        for i in 0..self.force_model_projection.nr_span_lines() {
            let span_line  = &span_lines_projection[i];
            let chord_vector = chord_vectors_projection[i];

            let sectional_force = sectional_forces_to_project[i];

            let effective_chord_vector = if self.settings.velocity_aligned_projection {
                result.velocity[i].normalize() * chord_vector.length()
            } else {
                chord_vector
            };

            let projection = self.projection.projection_value_at_point(point, effective_chord_vector, &span_line);

            body_force += sectional_force * projection;
        }

        body_force
    }

    pub fn distributed_body_force_weight_at_point(&self, point: Vec3) -> f64 {
        let mut body_force_weight = 0.0;

        let span_lines_projection = self.force_model_projection.span_lines();
        let chord_vectors_projection = self.force_model_projection.chord_vectors();

        let result = self.results.last().unwrap();

        for i in 0..self.force_model_projection.nr_span_lines() {
            let span_line  = &span_lines_projection[i];
            let chord_vector = chord_vectors_projection[i];

            let effective_chord_vector = if self.settings.velocity_aligned_projection {
                result.velocity[i].normalize() * chord_vector.length()
            } else {
                chord_vector
            };

            body_force_weight += self.projection.projection_value_at_point(point, effective_chord_vector, &span_line);
        }

        body_force_weight
    }
}