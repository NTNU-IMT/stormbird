// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! An interface for running simulations using the lifting line models.
//!
//! To support different use cases, both quasi-steady and true dynamic simulations are supported.
//! However, the interface is designed to be as unified as possible.

use stormath::type_aliases::Float;

use crate::lifting_line::prelude::*;

use crate::line_force_model::global_geometry_data::GlobalLineForceModelGeometry;

use super::simulation_builder::SimulationBuilder;

use crate::error::Error;

#[derive(Debug, Clone)]
/// Struct that contains the data needed to run a lifting line simulation.
pub struct Simulation {
    pub line_force_model: LineForceModel,
    pub flow_derivatives: FlowDerivatives,
    pub wake: Wake,
    pub frozen_wake: FrozenWake,
    pub solver: SimpleIterative,
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
        let total_nr_points = self.line_force_model.nr_span_lines() + self.wake.points.len();

        let mut points = Vec::with_capacity(total_nr_points);

        points.extend(self.line_force_model.ctrl_points());

        for i in 0..self.wake.points.len() {
            points.push(self.wake.points[i]);
        }

        points
    }

    pub fn initialize(
        &mut self, 
        line_force_model_geometry: &GlobalLineForceModelGeometry, 
        ctrl_points_freestream: &[SpatialVector],
        time_step: Float
    ) {
        let averaged_ctrl_points_freestream = ctrl_points_freestream.iter()
                .sum::<SpatialVector>() / ctrl_points_freestream.len() as Float;

        for i in 0..self.wake.velocity_at_points.len() {
            self.wake.velocity_at_points[i] = averaged_ctrl_points_freestream;
        }

        self.wake.initialize_with_velocity_and_time_step(
            line_force_model_geometry,
            averaged_ctrl_points_freestream,
            time_step,
        );

        self.flow_derivatives = FlowDerivatives::new(
            &ctrl_points_freestream
        );

        self.first_time_step_completed = true;
        
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
        let ctrl_points_freestream = freestream_velocity[
            0..self.line_force_model.nr_span_lines()
        ].to_vec();

        let wake_points_freestream = freestream_velocity[
            self.line_force_model.nr_span_lines()..
        ].to_vec();

        let felt_ctrl_points_freestream = self.line_force_model.felt_ctrl_points_velocity(
            &ctrl_points_freestream
        );

        let felt_span_points_freestream = self.line_force_model.span_point_values_from_ctrl_point_values(
            &felt_ctrl_points_freestream, 
            false
        );

        let line_force_model_geometry = GlobalLineForceModelGeometry::new(
            &self.line_force_model
        );

        if !self.first_time_step_completed {
            self.initialize(&line_force_model_geometry, &ctrl_points_freestream, time_step);
        }

        // Update the full wake model based on the current state
        self.wake.update_before_solving(
            time_step, 
            &line_force_model_geometry, 
            &felt_span_points_freestream
        );

        // Use the updated wake model to update the frozen wake
        self.frozen_wake.update(
            &line_force_model_geometry.ctrl_points,
            &self.wake
        );

        // Run the solver with the frozen wake
        let solver_result = self.solver.do_step(
            &self.line_force_model,
            &felt_ctrl_points_freestream,
            &self.frozen_wake,
            &self.previous_circulation_strength
        );

        // Update the wake model after solving
        self.wake.update_after_solving(
            &solver_result.circulation_strength,
            &wake_points_freestream,
        );

        let time_step_index = (time / time_step) as usize;

        self.wake.write_wake_data_to_file_if_activated(time_step_index);

        self.previous_circulation_strength = solver_result.circulation_strength.clone();

        let ctrl_point_acceleration = self.flow_derivatives.acceleration(
            &solver_result.output_ctrl_point_velocity, 
            time_step
        );
        
        self.flow_derivatives.update(&solver_result.output_ctrl_point_velocity);

        self.line_force_model.calculate_simulation_result(&solver_result, &ctrl_point_acceleration, time)
    }


    /// Interface function to calculate the induced velocities from the wake at the given points.
    pub fn induced_velocities(
        &self,
        points: &[SpatialVector],
    ) -> Vec<SpatialVector> {
        self.wake.induced_velocities(points)
    }
}
