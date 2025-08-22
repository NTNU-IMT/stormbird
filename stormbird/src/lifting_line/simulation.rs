// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! An interface for running simulations using the lifting line models.
//!
//! To support different use cases, both quasi-steady and true dynamic simulations are supported.
//! However, the interface is designed to be as unified as possible.

use crate::lifting_line::prelude::*;
use crate::line_force_model::corrections::circulation::{
    CirculationCorrection,
    prescribed::PrescribedCirculation,
};

use super::simulation_builder::SimulationBuilder;

use super::wake::line_force_model_data::LineForceModelData;

use crate::error::Error;

#[derive(Debug, Clone)]
/// Struct that contains the data needed to run a lifting line simulation.
pub struct Simulation {
    pub line_force_model: LineForceModel,
    pub flow_derivatives: FlowDerivatives,
    pub wake: Wake,
    pub frozen_wake: FrozenWake,
    pub solver: SimpleIterative,
    pub previous_circulation_strength: Vec<f64>,
    pub previous_line_force_model_data: LineForceModelData,
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
    /// Which points that are returned depends on the simulation mode. In case of a quasi-steady
    /// simulation, the points are only the control points of the line force model. In case of a
    /// dynamic simulation, the points are the control points of the line force model and the
    /// points in the wake.
    pub fn get_freestream_velocity_points(&self) -> Vec<SpatialVector> {
        let total_nr_popints = self.line_force_model.nr_span_lines() + self.wake.points.len();

        let mut points = Vec::with_capacity(total_nr_popints);

        points.extend(self.line_force_model.ctrl_points());

        for i in 0..self.wake.points.len() {
            points.push(self.wake.points[i]);
        }

        points
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
        time: f64,
        time_step: f64,
        freestream_velocity: &[SpatialVector],
    ) -> SimulationResult {
        let ctrl_points_freestream = freestream_velocity[0..self.line_force_model.nr_span_lines()].to_vec();
        let wake_points_freestream = freestream_velocity[self.line_force_model.nr_span_lines()..].to_vec();

        let felt_ctrl_points_freestream = self.line_force_model.felt_ctrl_points_velocity(
            &ctrl_points_freestream
        );

        if !self.first_time_step_completed {
            let averaged_ctrl_points_freesteream = ctrl_points_freestream.iter()
                .sum::<SpatialVector>() / ctrl_points_freestream.len() as f64;

            self.wake.initialize_with_velocity_and_time_step(
                &self.line_force_model,
                averaged_ctrl_points_freesteream,
                time_step,
            );

            self.initialize_line_force_model_data(&felt_ctrl_points_freestream);

            self.first_time_step_completed = true;
        }

        self.wake.update_before_solving(
            time_step, 
            &self.line_force_model, 
            &self.previous_line_force_model_data
        );

        self.frozen_wake.update(
            &self.wake
        );

        let solver_result = self.solver.do_step(
            &self.line_force_model,
            &felt_ctrl_points_freestream,
            &self.frozen_wake,
            &self.previous_circulation_strength
        );

        self.wake.update_after_solving(
            &solver_result.circulation_strength,
            &wake_points_freestream,
        );

        let time_step_index = (time / time_step) as usize;

        self.wake.write_wake_data_to_file_if_activated(time_step_index);

        self.previous_circulation_strength = solver_result.circulation_strength.clone();

        self.previous_line_force_model_data = LineForceModelData::new(
            &self.line_force_model,
            &felt_ctrl_points_freestream,
            &solver_result.ctrl_point_velocity,
        );

        self.flow_derivatives.update(&solver_result.ctrl_point_velocity);

        self.line_force_model.calculate_simulation_result(&solver_result, time, time_step)
    }


    /// Interface function to calculate the induced velocities from the wake at the given points.
    pub fn induced_velocities(
        &self,
        points: &[SpatialVector],
    ) -> Vec<SpatialVector> {
        self.wake.induced_velocities(points)
    }

    pub fn initialize_line_force_model_data(
        &mut self,
        felt_ctrl_points_freestream: &[SpatialVector],
    ) {
        self.previous_line_force_model_data = LineForceModelData::new(
            &self.line_force_model,
            felt_ctrl_points_freestream,
            felt_ctrl_points_freestream,
        );
    }

    /// Initialize the circulation strength of the simulation with a given elliptic distribution.
    pub fn initialize_with_elliptic_distribution(
        &mut self,
        time: f64,
        time_step: f64,
        freestream_velocity: &[SpatialVector],
    ) {
        let old_circulation_correction = self.line_force_model.circulation_correction.clone();
        let old_damping_factor = self.solver.damping_factor;

        let ctrl_points_freestream = freestream_velocity[
            0..self.line_force_model.nr_span_lines()
        ].to_vec();

        let felt_ctrl_points_freestream = self.line_force_model.felt_ctrl_points_velocity(
            &ctrl_points_freestream
        );

        self.initialize_line_force_model_data(&felt_ctrl_points_freestream);

        self.line_force_model.circulation_correction = CirculationCorrection::Prescribed(
            PrescribedCirculation::default()
        );
        self.solver.damping_factor = 0.25_f64.max(old_damping_factor);

        let _ = self.do_step(time, time_step, freestream_velocity);

        self.line_force_model.circulation_correction = old_circulation_correction;
        self.solver.damping_factor = old_damping_factor;
    }
}
