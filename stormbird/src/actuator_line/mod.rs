// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Implementation of actuator line functionality.

use std::fs;
use std::path::Path;

pub mod projection;
pub mod sampling;
pub mod builder;
pub mod solver;
pub mod corrections;

use stormath::smoothing::gaussian::gaussian_kernel;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use crate::line_force_model::LineForceModel;

use crate::common_utils::prelude::*;
use crate::controller::prelude::*;
use crate::wind::environment::WindEnvironment;

use crate::io_utils;

use projection::ProjectionSettings;
use sampling::SamplingSettings;
use builder::ActuatorLineBuilder;
use solver::SolverSettings;

use corrections::{
    lifting_line::LiftingLineCorrection,
    empirical_circulation::EmpiricalCirculationCorrection,
};

#[derive(Debug, Clone)]
/// Structure for representing an actuator line model.
pub struct ActuatorLine {
    /// The line force model used to compute forces on each line segment as a function of the local
    /// velocity.
    pub line_force_model: LineForceModel,
    /// Enum, with an internal structure, that determines how forces are projected in a CFD
    /// simulation
    pub projection_settings: ProjectionSettings,
    /// Settings for the solver
    pub solver_settings: SolverSettings,
    /// Settings for the velocity sampling
    pub sampling_settings: SamplingSettings,
    /// Dynamic optimizer that can be optionally used to optimize the settings in the model
    pub controller: Option<Controller>,
    /// The time to start solving
    pub start_time: Float,
    /// The current iteration of the simulation
    pub current_iteration: usize,
    /// The number of iterations between each time a full simulation result is written to file
    pub write_iterations_full_result: usize,
    /// Vector to store interpolated velocity values for each control point
    pub ctrl_points_velocity: Vec<SpatialVector>,
    /// Results from the model
    pub simulation_result: Option<SimulationResult>,
    /// Lift forces that are projected back to the CFD domain
    pub sectional_lift_forces_to_project: Vec<SpatialVector>,
    /// Drag forces that are projected back to the CFD domain
    pub sectional_drag_forces_to_project: Vec<SpatialVector>,
    /// Corrections based on the lifting line model
    pub lifting_line_correction: Option<LiftingLineCorrection>,
    /// Empirical correction for the circulation strength, also known as a tip loss factor
    pub empirical_circulation_correction: Option<EmpiricalCirculationCorrection>,
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
    /// a given cell. This is used when estimating the velocity at control points using the integral
    /// method.
    pub fn get_weighted_velocity_sampling_integral_terms_for_cell(
        &self,
        line_index: usize,
        velocity: SpatialVector,
        cell_center: SpatialVector,
        cell_volume: Float
    ) -> (SpatialVector, Float) {
        let span_line = self.line_force_model.span_lines_global[line_index];
        let chord_vector = self.line_force_model.chord_vectors_global[line_index];

        let projection_value_org = self.projection_settings.projection_value_at_point(
            cell_center, chord_vector, &span_line
        );

        let projection_value = if projection_value_org > 0.0 {
            let line_coordinates = span_line.line_coordinates(cell_center, chord_vector);

            let span_projection = if self.sampling_settings.neglect_span_projection {
                1.0
            } else {
                let span_smoothing_length = self.sampling_settings.span_projection_factor * 
                    span_line.length();

                gaussian_kernel(
                    line_coordinates.span,
                    0.0,
                    span_smoothing_length
                )
            };

            projection_value_org * span_projection
        } else {
            0.0
        };

        let denominator = cell_volume * projection_value;
        let numerator = velocity * denominator;

        (numerator, denominator)
    }


    /// Function to be executed at each time step in the CFD simulation.
    ///
    /// It solves for the circulation strength and computes the simulation result based on the
    /// current estimate of the control point velocities.
    pub fn do_step(&mut self, time: Float, time_step: Float){
        if time >= self.start_time {
            let solver_result = self.solve(time, time_step);

            let ctrl_point_acceleration = vec![
                SpatialVector::default();
                self.line_force_model.nr_span_lines()
            ];

            let simulation_result = self.line_force_model.calculate_simulation_result(
                &solver_result,
                &ctrl_point_acceleration,
                time,
            );
            
            //self.line_force_model.update_flow_derivatives(&result);

            self.simulation_result = Some(simulation_result);
            
            self.update_sectional_forces_to_project();
        }

        self.current_iteration += 1;
    }

    /// Function to update the controller in the model, if the controller is present.
    pub fn update_controller(&mut self, time: Float, time_step: Float) -> bool {
        if time >= self.start_time {
            let controller_output = if let Some(controller) = &mut self.controller {
                let simulation_result = self.simulation_result.as_ref().unwrap();

                let wind_environment = WindEnvironment::default();

                let input = ControllerInput::new_from_simulation_result(
                    1.0,
                    &self.line_force_model,
                    &simulation_result,
                    &controller.flow_measurement_settings,
                    &wind_environment,
                    false
                );

                controller.update(time, time_step, &input)
            } else {
                None
            };

            let mut need_update = false;

            if let Some(controller_output) = controller_output {
                self.line_force_model.set_controller_output(&controller_output);

                need_update = true;

                ControllerOutput::write_to_csv_file(&controller_output, "controller_output.csv");
            }

            need_update
        } else {
            false
        }
    }

    /// Computes a corrected velocity at the control points, based on the sampling settings, and,
    /// if present, the lifting line correction.
    pub fn corrected_ctrl_points_velocity(&self, time: Float) -> Vec<SpatialVector> {
        let mut corrected_velocity = if self.sampling_settings.remove_span_velocity {
            self.line_force_model.remove_span_velocity(
                &self.ctrl_points_velocity,
                CoordinateSystem::Global
            )
        } else {
            self.ctrl_points_velocity.clone()
        };

        if let Some(lifting_line_correction) = &self.lifting_line_correction {
            let last_circulation_strength = if let Some(result) = &self.simulation_result {
                result.force_input.circulation_strength.clone()
            } else {
                vec![0.0; self.line_force_model.nr_span_lines()]
            };

            let ll_velocity_correction = lifting_line_correction.velocity_correction(
                &self.line_force_model,
                &self.ctrl_points_velocity,
                &last_circulation_strength,
                time - self.start_time
            );

            for i in 0..corrected_velocity.len() {
                corrected_velocity[i] += ll_velocity_correction[i];
            }
        }

        for i in 0..corrected_velocity.len() {
            corrected_velocity[i] *= self.sampling_settings.correction_factor;
        }

        if self.sampling_settings.extrapolate_end_velocities {
            for wing_index in 0..self.line_force_model.nr_wings() {
                let first_line_index = self.line_force_model.wing_indices[wing_index].start;
                let second_line_index = first_line_index + 1;
                let third_line_index = second_line_index + 1;

                let last_line_index = self.line_force_model.wing_indices[wing_index].end - 1;
                let second_to_last_line_index = last_line_index - 1;
                let third_to_last_line_index = second_to_last_line_index - 1;

                let first_delta_velocity =
                    corrected_velocity[second_line_index] -
                    corrected_velocity[third_line_index];
                let last_delta_velocity =
                    corrected_velocity[second_to_last_line_index] -
                    corrected_velocity[third_to_last_line_index];

                corrected_velocity[first_line_index] =
                    corrected_velocity[second_line_index] +
                    first_delta_velocity;
                corrected_velocity[last_line_index] =
                    corrected_velocity[second_to_last_line_index] +
                    last_delta_velocity;
            }
        }

        corrected_velocity
    }

    /// Takes the estimated velocity on at the control points as input and calculates a simulation
    /// result from the line force model.
    pub fn solve(&mut self, time: Float, _time_step: Float) -> SolverResult {
        let corrected_ctrl_points_velocity = self.corrected_ctrl_points_velocity(time);

        let angles_of_attack = self.line_force_model.angles_of_attack(
            &corrected_ctrl_points_velocity,
            CoordinateSystem::Global
        );

        let mut new_estimated_circulation_strength = self.line_force_model.circulation_strength(
            &angles_of_attack,
            &corrected_ctrl_points_velocity
        );

        if let Some(empirical_circulation_correction) = &self.empirical_circulation_correction {
            let non_dim_span_positions = &self.line_force_model.ctrl_point_spanwise_distance_circulation_model;

            for i in 0..new_estimated_circulation_strength.len() {
                new_estimated_circulation_strength[i] *= empirical_circulation_correction.correction_factor(
                    non_dim_span_positions[i]
                );
            }
        }

        let previous_strength = if let Some(simulation_result) = &self.simulation_result {
            simulation_result.force_input.circulation_strength.clone()
        } else {
            vec![0.0; self.line_force_model.nr_span_lines()]
        };

        let mut circulation_strength = Vec::with_capacity(new_estimated_circulation_strength.len());
        for i in 0..new_estimated_circulation_strength.len() {
            let strength_difference = new_estimated_circulation_strength[i] - previous_strength[i];

            circulation_strength.push(
                previous_strength[i] + self.solver_settings.damping_factor * strength_difference
            );
        }

        let residual = self.line_force_model.average_residual_absolute(
            &angles_of_attack,
            &circulation_strength,
            &corrected_ctrl_points_velocity
        );

        SolverResult {
            input_ctrl_points_velocity: self.ctrl_points_velocity.clone(),
            circulation_strength,
            output_ctrl_points_velocity: corrected_ctrl_points_velocity,
            iterations: 1,
            residual,
        }
    }

    /// Writes the resulting values from the line force model to a file.
    pub fn write_results(&self, folder_path: &str) {
        if let Some(simulation_result) = &self.simulation_result {
            let overall_folder_path = Path::new(folder_path);

            io_utils::folder_management::ensure_folder_exists(overall_folder_path).unwrap();

            let (header, data) = simulation_result.as_reduced_flatten_csv_string();

            let force_file_path = format!("{}/stormbird_forces.csv", folder_path);

            let _ = io_utils::csv_data::create_or_append_header_and_data_strings_file(
                &force_file_path,
                &header,
                &data
            );

            if self.current_iteration % self.write_iterations_full_result == 0 {
                let result_folder_path = Path::new(folder_path).join("stormbird_full_results");
                io_utils::folder_management::ensure_folder_exists(&result_folder_path).unwrap();

                let json_string = serde_json::to_string_pretty(&simulation_result).unwrap();

                io_utils::write_text_to_file(
                    format!(
                        "{}/full_results_{}.json",
                        result_folder_path.to_str().unwrap(),
                        self.current_iteration
                    ).as_str(),
                    &json_string
                ).unwrap();
            }
        }
    }
    
    /// Function 
    pub fn update_sectional_forces_to_project(&mut self) {
        let nr_span_lines = self.line_force_model.nr_span_lines();
        
        if let Some(simulation_result) = &self.simulation_result {
            for line_index in 0..nr_span_lines {
                let mut lift_force = simulation_result.sectional_forces.circulatory[line_index];
                let mut drag_force = SpatialVector::default();
                
                if self.projection_settings.project_viscous_lift {
                    lift_force += simulation_result.sectional_forces.viscous_lift[line_index];
                }
                
                if self.projection_settings.project_sectional_drag {
                    drag_force = simulation_result.sectional_forces.sectional_drag[line_index];
                }
                
                if self.projection_settings.realign_sectional_forces {
                    let line = self.line_force_model.span_lines_global[line_index];
                    let velocity = self.ctrl_points_velocity[line_index];
    
                    let lift_direction = line
                        .relative_vector()
                        .cross(velocity)
                        .normalize();
                    
                    let drag_direction = velocity.normalize();
                    
                    lift_force = lift_force.length() * lift_direction;
                    drag_force = drag_force.length() * drag_direction;
                }
                
                self.sectional_lift_forces_to_project[line_index] = lift_force;
                self.sectional_drag_forces_to_project[line_index] = drag_force;
                
            }
        } else {
            self.sectional_lift_forces_to_project = vec![SpatialVector::default(); nr_span_lines];
            self.sectional_drag_forces_to_project = vec![SpatialVector::default(); nr_span_lines];
        }
        
    }

    /// Returns the force to be projected, based on the line index
    ///
    /// # Arguments
    /// * `line_index` - The index of the line segment for which the force is to be projected.
    /// * `velocity` - The velocity vector at the control point of the cell where the force is to be
    /// projected.
    pub fn force_to_project_at_cell(
        &self,
        line_index: usize,
        velocity: SpatialVector
    ) -> SpatialVector {
        let lift_force = self.sectional_lift_forces_to_project[line_index];
        let drag_force = self.sectional_drag_forces_to_project[line_index];

        if self.projection_settings.realign_to_local_velocity_at_each_cell {
            let line = self.line_force_model.span_lines_global[line_index];

            let lift_direction = line
                .relative_vector()
                .cross(velocity)
                .normalize();

            let drag_direction = velocity.normalize();

            lift_force.length() * lift_direction + drag_force.length() * drag_direction
        } else {
            lift_force + drag_force
        }
    }

    /// Computes the body force weights for each line element at a given point in space.
    pub fn line_segments_projection_weights_at_point(&self, point: SpatialVector) -> Vec<Float> {
        let span_lines = &self.line_force_model.span_lines_global;
        let chord_vectors = &self.line_force_model.chord_vectors_global;

        let mut projection_values = Vec::with_capacity(self.line_force_model.nr_span_lines());

        for i in 0..self.line_force_model.nr_span_lines() {
            projection_values.push(
                self.projection_settings.projection_value_at_point(
                    point,
                    chord_vectors[i],
                    &span_lines[i]
                )
            );
        }

        projection_values
    }

    /// Computes the sum of the projection weights for all line elements at a given point in space.
    pub fn summed_projection_weights_at_point(&self, point: SpatialVector) -> Float {
        self.line_segments_projection_weights_at_point(point).iter().sum()
    }

    /// Checks which line element is dominating at a given point in space by comparing the
    /// projection weights of each line element.
    pub fn dominating_line_element_index_at_point(&self, point: SpatialVector) -> usize {
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
