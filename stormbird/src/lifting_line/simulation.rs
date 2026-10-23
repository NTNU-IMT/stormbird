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
    /// The line force model representing the wings
    pub line_force_model: LineForceModel,
    /// History of the flow at the control points, used to compute the flow acceleration
    pub flow_derivatives: FlowDerivatives,
    /// The wake model, either quasi-steady or dynamic
    pub wake_data: WakeData,
    /// Frozen version of the wake, used to compute induced velocities efficiently in the solver
    pub frozen_wake: FrozenWake,
    /// The solver used to find the circulation strength
    pub solver: Solver,
    /// The circulation strength from the previous time step, used as initial solution in the next
    pub previous_circulation_strength: Vec<Float>,
    /// Whether the first time step is completed, i.e., whether the simulation is initialized
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

    /// Initializes the simulation before the first time step, based on the freestream velocity at
    /// the control points
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
            ctrl_points_freestream
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
    ///   `get_freestream_velocity_points`
    pub fn do_step(
        &mut self,
        time: Float,
        time_step: Float,
        freestream_velocity: &[SpatialVector],
    ) -> SimulationResult {
        self.line_force_model.update_global_data_representations();

        let ctrl_points_freestream_range = 0..self.line_force_model.nr_span_lines();
        let wake_points_freestream_range = self.line_force_model.nr_span_lines()..freestream_velocity.len();

        let ctrl_points_freestream = &freestream_velocity[ctrl_points_freestream_range.clone()];

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
                    ctrl_points_freestream
                );

                // Use the updated wake model to update the frozen wake
                self.frozen_wake.update_from_full_wake(
                    &self.line_force_model.ctrl_points_global,
                    wake
                );
            },
            WakeData::QuasiSteady(wake) => {
                // Rebuild the horseshoe vortices for the current state
                wake.update_geometry(
                    &self.line_force_model,
                    ctrl_points_freestream
                );

                self.frozen_wake.update_from_quasi_steady_wake(
                    &self.line_force_model.ctrl_points_global,
                    wake
                );
            }
        }

        // Run the solver with the frozen wake

        let felt_ctrl_points_freestream = self.line_force_model.felt_ctrl_points_velocity(
            ctrl_points_freestream
        );

        let solver_result = match &self.solver {
            Solver::Iterative(solver) => solver.solve(
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

        // Update the wake model after solving
        match &mut self.wake_data {
            WakeData::Dynamic(wake) => {
                wake.update_after_solving(
                    &solver_result.circulation_strength,
                    &freestream_velocity[wake_points_freestream_range.clone()],
                );
            },
            WakeData::QuasiSteady(wake) => {
                wake.update_strengths(&solver_result.circulation_strength);
            }
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

    /// Serialize the current wake as a legacy VTK PolyData buffer, so the wake geometry and
    /// strengths can be obtained in memory (e.g. for visualization in pyvista) without touching the
    /// disk. The `binary` switch selects between the ASCII and BINARY variants of the VTK legacy
    /// format.
    ///
    /// Works for both dynamic and quasi-steady wakes. A dynamic wake is exported as one quad per
    /// panel, while a quasi-steady wake is exported as one polygon per horseshoe vortex. The wake
    /// represents the state at the last completed time step. A quasi-steady wake is empty before
    /// the first time step.
    pub fn wake_as_vtk(&self, binary: bool) -> Vec<u8> {
        self.wake_data.wake_as_vtk(binary)
    }

    #[deprecated(note = "Use `wake_as_vtk`, which works for both dynamic and quasi-steady wakes")]
    /// Serialize the current wake as a legacy VTK PolyData buffer. Kept for backwards
    /// compatibility, and always returns `Some`, as both dynamic and quasi-steady wakes can now be
    /// exported.
    pub fn dynamic_wake_as_vtk(&self, binary: bool) -> Option<Vec<u8>> {
        Some(self.wake_as_vtk(binary))
    }

    /// Interface function to calculate the induced velocities from the wake at the given points.
    ///
    /// Works for both dynamic and quasi-steady wakes, and uses the state of the wake at the last
    /// completed time step. A quasi-steady wake gives zero induced velocities before the first time
    /// step.
    pub fn induced_velocities(
        &self,
        points: &[SpatialVector],
    ) -> Vec<SpatialVector> {
        self.wake_data.induced_velocities(points)
    }
}
