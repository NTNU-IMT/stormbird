// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! An interface for running simulations using the lifting line models.
//! 
//! To support different use cases, both quasi-steady and true dynamic simulations are supported. 
//! However, the interface is designed to be as unified as possible. 

use serde::{Serialize, Deserialize};

use crate::lifting_line::prelude::*;
use crate::io_structs::derivatives::Derivatives;


#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a quasi-steady simulation.
pub struct SteadySettings {
    #[serde(default)]
    pub solver: SteadySolverSettings,
    #[serde(default)]
    pub wake: SteadyWakeBuilder,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
/// Settings for a dynamic simulation.
pub struct UnsteadySettings {
    #[serde(default)]
    pub solver: UnsteadySolverSettings,
    #[serde(default)]
    pub wake: UnsteadyWakeBuilder,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
/// Enum that determines what type of simulation to run. 
/// 
/// Both quasi-steady and dynamic simulations are supported. The settings for each simulation type 
/// is stored as a member variable of each variant.
pub enum SimulationMode {
    QuasiSteady(SteadySettings),
    Dynamic(UnsteadySettings),
}

#[derive(Debug, Clone)]
pub enum WakeModel {
    Steady(SteadyWake),
    Unsteady(UnsteadyWake)
}

impl Default for SimulationMode {
    fn default() -> Self {
        SimulationMode::QuasiSteady(SteadySettings::default())
    }

}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
/// Builder for the [Simulation] struct.
pub struct SimulationBuilder {
    pub line_force_model: LineForceModelBuilder,
    #[serde(default)]
    pub simulation_mode: SimulationMode,
    #[serde(default)]
    pub write_wake_data_to_file: bool,
    #[serde(default)]
    pub wake_files_folder_path: String,
}

impl SimulationBuilder {
    pub fn new(
        line_force_model: LineForceModelBuilder,
        simulation_mode: SimulationMode,
    ) -> Self {
        SimulationBuilder {
            line_force_model,
            simulation_mode,
            write_wake_data_to_file: false,
            wake_files_folder_path: String::new()
        }
    }

    /// Creates a new simulation builder by parsing the the string as a JSON object. The parsing is
    /// done using the serde_json library.
    pub fn new_from_string(string: &str) -> Result<Self, String> {
        match serde_json::from_str(string) {
            Ok(builder) => Ok(builder),
            Err(e) => Err(format!("Error parsing JSON: {}", e))
        }
    }

    /// Creates a new simulation builder by reading the file at the given path and parsing the 
    /// content as a JSON object. The parsing is done using the [SimulationBuilder::new_from_string]
    /// method.
    pub fn new_from_file(file_path: &str) -> Result<Self, String> {
        match std::fs::read_to_string(file_path) {
            Ok(string) => Self::new_from_string(&string),
            Err(e) => Err(format!("Error reading file: {}", e))
        }
    }

    /// Builds the [Simulation] struct based on the current state of the builder.
    pub fn build(&self, initial_time_step: f64, wake_initial_velocity: Vec3) -> Simulation {
        let line_force_model = self.line_force_model.build();
        let nr_of_lines = line_force_model.nr_span_lines();

        let wake_model = match &self.simulation_mode {
            SimulationMode::Dynamic(settings) => {
                WakeModel::Unsteady(
                    settings.wake.build(
                        initial_time_step,
                        &line_force_model,
                        wake_initial_velocity,
                    )
                )
            },
            SimulationMode::QuasiSteady(settings) => {
                let ctrl_points_freestream = vec![wake_initial_velocity; nr_of_lines];

                WakeModel::Steady(
                    settings.wake.build(
                        &line_force_model,
                        &ctrl_points_freestream
                    )
                )
            }
        };

        Simulation {
            line_force_model,
            simulation_mode: self.simulation_mode.clone(),
            derivatives: None,
            previous_circulation_strength: vec![0.0; nr_of_lines],
            wake_model,
            write_wake_data_to_file: self.write_wake_data_to_file,
            wake_files_folder_path: self.wake_files_folder_path.clone()
        }
    }
}

#[derive(Debug, Clone)]
/// Struct that contains the data needed to run a dynamic simulation.
pub struct Simulation {
    pub line_force_model: LineForceModel,
    simulation_mode: SimulationMode,
    previous_circulation_strength: Vec<f64>,
    derivatives: Option<Derivatives>,
    wake_model: WakeModel,
    write_wake_data_to_file: bool,
    wake_files_folder_path: String,
}

impl Simulation {
    pub fn new_from_string(setup_string: &str, initial_time_step: f64, wake_initial_velocity: Vec3) -> Result<Self, String> {
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
    pub fn get_freestream_velocity_points(&self) -> Vec<Vec3> {
        match &self.simulation_mode {
            SimulationMode::QuasiSteady(_) => {
                self.line_force_model.ctrl_points()
            },
            SimulationMode::Dynamic(_) => {
                let mut points = self.line_force_model.ctrl_points();

                if let WakeModel::Unsteady(wake) = &self.wake_model {

                    let wake_points = &wake.wake_points;

                    for i in 0..wake_points.len() {
                        points.push(wake_points[i]);
                    }
                } else {
                    panic!("The wake model is not of the unsteady type.");
                }

                points
            }
        }
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
        freestream_velocity: &[Vec3],
    ) -> SimulationResult {
        let ctrl_points_freestream = freestream_velocity[0..self.line_force_model.nr_span_lines()].to_vec();

        let wake_points_freestream: Option<Vec<Vec3>> = match &self.simulation_mode {
            SimulationMode::QuasiSteady(_) => {
                None
            },
            SimulationMode::Dynamic(_) => {
                Some(freestream_velocity[self.line_force_model.nr_span_lines()..].to_vec())
            }
        };

        // If the force input calculator has not been initialized, initialize it.
        if self.derivatives.is_none() {
            let initial_angles = self.line_force_model.angles_of_attack(&ctrl_points_freestream);

            self.derivatives = Some(
                Derivatives::new(
                    &self.line_force_model,
                    &ctrl_points_freestream,
                    &initial_angles
                )
            );
        }

        let derivatives = self.derivatives.as_mut().unwrap();

        let solver_result = match &self.simulation_mode {
            SimulationMode::QuasiSteady(settings) => {
                match &mut self.wake_model {
                    WakeModel::Unsteady(_) => {
                        panic!("The wake model is not of the steady type.");
                    },
                    WakeModel::Steady(wake) => {
                        *wake = settings.wake.build(
                            &self.line_force_model,
                            &ctrl_points_freestream
                        );

                        steady_solvers::solve_steady(
                            time_step,
                            &self.line_force_model, 
                            &ctrl_points_freestream,
                            Some(&derivatives),
                            &settings.solver,
                            &settings.wake, 
                            &self.previous_circulation_strength
                        )
                    }
                }
            },
            SimulationMode::Dynamic(settings) => {
                match &mut self.wake_model {
                    WakeModel::Steady(_) => {
                        panic!("The wake model is not of the unsteady type.");
                    },
                    WakeModel::Unsteady(wake) => {
                        let result = unsteady_solvers::solve_one_time_step(
                            time_step,
                            &self.line_force_model,
                            &ctrl_points_freestream,
                            &wake_points_freestream.unwrap(),
                            &derivatives,
                            wake,
                            &settings.solver,
                            &self.previous_circulation_strength
                        );
    
                        let time_step_index = (time / time_step) as usize;
    
                        if self.write_wake_data_to_file {
                            let wake_file_path = format!("{}/wake_{}.vtp", self.wake_files_folder_path, time_step_index);
    
                            let write_result = wake.write_wake_to_vtk_file(&wake_file_path);
    
                            match write_result {
                                Ok(_) => {},
                                Err(e) => {
                                    println!("Error writing wake data to file: {}", e);
                                }
                            }
                        }
    
                        result
                    }
                }
            }
        };

        let angles_of_attack = self.line_force_model.angles_of_attack(&solver_result.ctrl_point_velocity);
        let acceleration = derivatives.flow.acceleration(&solver_result.ctrl_point_velocity, time_step);
        let angles_of_attack_derivative = derivatives.flow.angles_of_attack_derivative(&angles_of_attack, time_step);
        let rotation_velocity = derivatives.motion.rotation_velocity(&self.line_force_model, time_step);

        let force_input = SectionalForcesInput {
            circulation_strength: solver_result.circulation_strength.clone(),
            velocity: solver_result.ctrl_point_velocity.clone(),
            angles_of_attack,
            acceleration,
            angles_of_attack_derivative,
            rotation_velocity
        };

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
        };

        self.previous_circulation_strength = result.force_input.circulation_strength.clone();

        derivatives.update(
            &self.line_force_model,
            &result.force_input.velocity,
            &result.force_input.angles_of_attack
        );

        result
    }

    pub fn induced_velocities(&self, points: &[Vec3], off_body: bool) -> Vec<Vec3> {
        match &self.wake_model {
            WakeModel::Steady(wake) => {
                wake.induced_velocities(&self.previous_circulation_strength, points)
            },
            WakeModel::Unsteady(wake) => {
                wake.induced_velocities(points, off_body)
            }
        }
    }
}