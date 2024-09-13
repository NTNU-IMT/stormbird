// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! An interface for running simulations using the lifting line models.
//! 
//! To support different use cases, both quasi-steady and true dynamic simulations are supported. 
//! However, the interface is designed to be as unified as possible. 

use crate::lifting_line::prelude::*;
use crate::io_structs::derivatives::Derivatives;
use crate::line_force_model::circulation_corrections::CirculationCorrection;

use super::simulation_builder::SimulationBuilder;

#[derive(Debug, Clone)]
/// Struct that contains the data needed to run a dynamic simulation.
pub struct Simulation {
    pub line_force_model: LineForceModel,
    pub wake: Wake,
    pub solver: SimpleIterative,
    pub previous_circulation_strength: Vec<f64>,
    pub derivatives: Option<Derivatives>,
    pub write_wake_data_to_file: bool,
    pub wake_files_folder_path: String,
}

impl Simulation {
    pub fn new_from_string(
        setup_string: &str, 
        initial_time_step: f64, 
        wake_initial_velocity: SpatialVector<3>
    ) -> Result<Self, String> {
        let builder = SimulationBuilder::new_from_string(setup_string)?;

        Ok(builder.build(initial_time_step, wake_initial_velocity))
    }

    /// Returns the points where the freestream velocity must be specified in order to execute a 
    /// `do_step` call. 
    /// 
    /// Which points that are returned depends on the simulation mode. In case of a quasi-steady 
    /// simulation, the points are only the control points of the line force model. In case of a 
    /// dynamic simulation, the points are the control points of the line force model and the 
    /// points in the wake.
    pub fn get_freestream_velocity_points(&self) -> Vec<SpatialVector<3>> {
        let mut points = self.line_force_model.ctrl_points();

        for i in 0..self.wake.wake_points.len() {
            points.push(self.wake.wake_points[i]);
        }

        points
    }

    /// Steps the simulation forward in time by one time step. 
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
        freestream_velocity: &[SpatialVector<3>],
    ) -> SimulationResult {
        let ctrl_points_freestream = freestream_velocity[0..self.line_force_model.nr_span_lines()].to_vec();
        let wake_points_freestream = freestream_velocity[self.line_force_model.nr_span_lines()..].to_vec();

        // If the force input calculator has not been initialized, initialize it.
        if self.line_force_model.derivatives.is_none() {
            self.line_force_model.initialize_derivatives(&ctrl_points_freestream);
        }

        let felt_ctrl_points_freestream = self.line_force_model.felt_ctrl_points_freestream(
            &ctrl_points_freestream, time_step
        );

        self.wake.synchronize_wing_geometry(&self.line_force_model);

        let frozen_wake = FrozenWake::from_wake(
            &self.line_force_model, 
            &self.wake
        );

        // Solve for the circulation strength
        let solver_result = self.solver.do_step(
            &self.line_force_model,
            &felt_ctrl_points_freestream,
            &frozen_wake,
            &self.previous_circulation_strength
        );

        // Update the wake model if needed
        self.wake.update_after_completed_time_step(
            &solver_result.circulation_strength, 
            time_step, 
            &self.line_force_model,
            &ctrl_points_freestream,
            &wake_points_freestream,
        );

        let time_step_index = (time / time_step) as usize;

        if self.write_wake_data_to_file {
            let wake_file_path = format!("{}/wake_{}.vtp", self.wake_files_folder_path, time_step_index);

            let write_result = self.wake.write_wake_to_vtk_file(&wake_file_path);

            match write_result {
                Ok(_) => {},
                Err(e) => {
                    println!("Error writing wake data to file: {}", e);
                }
            }
        }

        let force_input = self.line_force_model.sectional_force_input(&solver_result, time_step);

        let ctrl_points = self.line_force_model.ctrl_points();
        let sectional_forces   = self.line_force_model.sectional_forces(&force_input);
        let integrated_forces = sectional_forces.integrate_forces(&self.line_force_model);
        let integrated_moments = sectional_forces.integrate_moments(&self.line_force_model);

        let result = SimulationResult {
            ctrl_points,
            force_input,
            sectional_forces,
            integrated_forces,
            integrated_moments,
            iterations: solver_result.iterations,
            residual: solver_result.residual,
        };

        self.previous_circulation_strength = result.force_input.circulation_strength.clone();
        
        self.line_force_model.update_derivatives(
            &result.force_input.velocity, 
            &result.force_input.angles_of_attack
        );

        result
    }

    pub fn induced_velocities(
        &self, 
        points: &[SpatialVector<3>], 
        off_body: bool
    ) -> Vec<SpatialVector<3>> {
        self.wake.induced_velocities(points, off_body)
    }

    pub fn initialize_with_elliptic_distribution(
        &mut self, 
        time: f64,
        time_step: f64,
        freestream_velocity: &[SpatialVector<3>],
    ) {
        let old_circulation_correction = self.line_force_model.circulation_corrections.clone();
        let old_damping_factor = self.solver.damping_factor;

        self.line_force_model.circulation_corrections =CirculationCorrection::PrescribedCirculation(PrescribedCirculationShape::default());
        self.solver.damping_factor = 0.25_f64.max(old_damping_factor);

        let _ = self.do_step(time, time_step, freestream_velocity);

        self.line_force_model.circulation_corrections = old_circulation_correction;
        self.solver.damping_factor = old_damping_factor;
    }
}