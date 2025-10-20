// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! An interface for running simulations using the lifting line models.
//!
//! To support different use cases, both quasi-steady and true dynamic simulations are supported.
//! However, the interface is designed to be as unified as possible.

use stormath::type_aliases::Float;

use crate::lifting_line::prelude::*;

use super::simulation_builder::SimulationBuilder;

use crate::error::Error;

#[derive(Debug, Clone)]
/// Struct that contains the data needed to run a lifting line simulation.
pub struct Simulation {
    pub line_force_model: LineForceModel,
    pub flow_derivatives: FlowDerivatives,
    pub wake_data: WakeData,
    pub frozen_wake: FrozenWake,
    pub solver: Solver,
    pub previous_circulation_strength: Vec<Float>,
    pub first_time_step_completed: bool,
}

impl Simulation {
    /// Creates a new simulation from a string that describes a `SimulationBuilder` in a JSON 
    /// format.
    pub fn new_from_string(
        setup_string: &str,
    ) -> Result<Self, Error> {
        let builder = SimulationBuilder::new_from_string(setup_string)?;

        Ok(builder.build())
    }

    /// Returns the points where the freestream velocity must be specified in order to execute a
    /// `do_step` call.
    ///
    /// The points consist of the ctrl points from the line force model, and all points in the wake
    pub fn get_freestream_velocity_points(&self) -> Vec<SpatialVector> {
        let mut total_nr_points = self.line_force_model.nr_span_lines();

        if let WakeData::Dynamic(wake) = &self.wake_data {
            total_nr_points += wake.points.len();
        };

        let mut points = Vec::with_capacity(total_nr_points);

        points.extend(self.line_force_model.ctrl_points_global.clone());

        if let WakeData::Dynamic(wake) = &self.wake_data {
            for i in 0..wake.points.len() {
                points.push(wake.points[i]);
            }
        }

        points
    }

    pub fn initialize(
        &mut self,
        ctrl_points_freestream: &[SpatialVector],
        time_step: Float
    ) {

        if let WakeData::Dynamic(wake) = &mut self.wake_data {
            let averaged_ctrl_points_freestream = ctrl_points_freestream.iter()
                .sum::<SpatialVector>() / ctrl_points_freestream.len() as Float;

            for i in 0..wake.velocity_at_points.len() {
                wake.velocity_at_points[i] = averaged_ctrl_points_freestream;
            }

            wake.initialize_with_velocity_and_time_step(
                &self.line_force_model,
                averaged_ctrl_points_freestream,
                time_step,
            );
        }
        
        self.flow_derivatives = FlowDerivatives::new(
            &ctrl_points_freestream
        );

        self.first_time_step_completed = true;
        

        self.previous_circulation_strength = vec![0.0; self.line_force_model.nr_span_lines()];
    }

    /// Steps the simulation forward in time by one time step.
    /// 
    /// # Steps that are performed in this function:
    /// - Updated the wake from the previous time step.
    /// - Solve for new circulation strength with updated wake structure
    ///
    /// # Arguments
    /// - `time`: The current time of the simulation.
    /// - `time_step`: The time step to use for the simulation.
    /// - `freestream_velocity`: The freestream velocity at the points returned by
    /// `get_freestream_velocity_points`
    pub fn do_step(
        &mut self,
        time: Float,
        time_step: Float,
        freestream_velocity: &[SpatialVector],
    ) -> SimulationResult {
        self.line_force_model.update_global_data_representations();

        let ctrl_points_freestream_range = 0..self.line_force_model.nr_span_lines();
        let wake_points_freestream_range = self.line_force_model.nr_span_lines()..freestream_velocity.len();

        let felt_ctrl_points_freestream = self.line_force_model.felt_ctrl_points_velocity(
            &freestream_velocity[ctrl_points_freestream_range.clone()],
        );

        let felt_span_points_freestream = self.line_force_model.span_point_values_from_ctrl_point_values(
            &felt_ctrl_points_freestream, 
            false
        );

        if !self.first_time_step_completed {
            self.initialize(
                &freestream_velocity[ctrl_points_freestream_range.clone()], 
                time_step
            );
        }

        match &mut self.wake_data {
            WakeData::Dynamic(wake) => {
                // Update the full wake model based on the current state
                wake.update_before_solving(
                    time_step, 
                    &self.line_force_model,
                    &felt_span_points_freestream
                );

                // Use the updated wake model to update the frozen wake
                self.frozen_wake.update_from_full_wake(
                    &self.line_force_model.ctrl_points_global,
                    &wake
                );
            },
            WakeData::QuasiSteady(settings) => {
                self.frozen_wake.update_as_steady_from_line_force_model_and_velocities(
                    &self.line_force_model, 
                    &felt_span_points_freestream,
                    &settings
                );
            }
        }

        // Run the solver with the frozen wake

        let solver_result = match &self.solver {
            Solver::SimpleIterative(solver) => solver.solve(
                &self.line_force_model,
                &felt_ctrl_points_freestream,
                &mut self.frozen_wake,
            &self.previous_circulation_strength
            ),
            Solver::Linearized(solver) => {
                solver.solve(
                    &self.line_force_model,
                    &felt_ctrl_points_freestream,
                    &mut self.frozen_wake,
                )
            }
        };

        if let WakeData::Dynamic(wake) = &mut self.wake_data {
            // Update the wake model after solving
            wake.update_after_solving(
                &solver_result.circulation_strength,
                &freestream_velocity[wake_points_freestream_range.clone()],
            );

            let time_step_index = (time / time_step) as usize;

            wake.write_wake_data_to_file_if_activated(time_step_index);
        }

    
        self.previous_circulation_strength = solver_result.circulation_strength.clone();

        let ctrl_point_acceleration = self.flow_derivatives.acceleration(
            &solver_result.output_ctrl_points_velocity, 
            time_step
        );
        
        self.flow_derivatives.update(&solver_result.output_ctrl_points_velocity);

        self.line_force_model.calculate_simulation_result(
            &solver_result, 
            &ctrl_point_acceleration, 
            time
        )
    }


    /// Interface function to calculate the induced velocities from the wake at the given points.
    pub fn induced_velocities(
        &self,
        points: &[SpatialVector],
    ) -> Vec<SpatialVector> {
        if let WakeData::Dynamic(wake) = &self.wake_data {
            return wake.induced_velocities(points);
        } else {
            panic!("Induced velocities not implemented for pure quasi-steady simulations yet");
        }
    }
}
