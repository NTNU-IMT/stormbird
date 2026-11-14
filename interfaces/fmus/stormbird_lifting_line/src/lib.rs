// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functional Mockup Unit (FMU) for dynamic lifting line simulations with the Stormbird library.
//!
//! The FMU is built around the [`CompleteSailModel`] from Stormbird, which is set up from a single
//! JSON file with parameters. See the README and the Stormbird book for more information.

mod error_codes;
mod input_filters;
mod parameters;
mod model_scaling;
mod setup;

use std::f64::consts::PI;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use fmu_from_struct::prelude::*;

use stormath::spatial_vector::SpatialVector;

use stormbird::common_utils::results::simulation::SimulationResult;
use stormbird::controller::input::ControllerInput;
use stormbird::lifting_line::complete_sail_model::CompleteSailModel;

use stormbird::wind::{
    wind_condition::WindCondition,
    wind_condition::velocity_variation::VelocityVariation,
    wind_condition::velocity_variation::power_model::PowerModel,
};

use error_codes::FmuError;
use input_filters::InputFilters;
use parameters::FmuParameters;
use model_scaling::ModelScaling;

/// The maximum number of sails, given by the number of inputs and outputs for each sail
pub const MAX_NR_SAILS: usize = 10;

// Exactly one version of the FMI standard must be selected with the cargo features
#[cfg(all(feature = "fmi2", feature = "fmi3"))]
compile_error!("The features 'fmi2' and 'fmi3' are mutually exclusive. Select only one of them.");

#[cfg(not(any(feature = "fmi2", feature = "fmi3")))]
compile_error!("Select the version of the FMI standard with either the 'fmi2' or the 'fmi3' feature.");

#[derive(Debug, Default, Clone, Fmu)]
#[cfg_attr(feature = "fmi2", fmu_from_struct(fmi_version = 2))]
#[cfg_attr(feature = "fmi3", fmu_from_struct(fmi_version = 3))]
/// FMU for a lifting line model using the Stormbird library.
pub struct StormbirdLiftingLine {
    #[fmu_from_struct(parameter)]
    /// Path to the parameters file. If empty, the parameters file is expected to be in the resource
    /// directory of the FMU.
    pub parameters_path: String,
    pub time_model_scale: f64,
    #[fmu_from_struct(input)]
    /// Variables specifying the wind conditions.
    pub wind_velocity: f64,
    pub wind_direction_coming_from: f64,
    /// Variables used to set the position and rotation of the model.
    pub translation_x: f64,
    pub translation_y: f64,
    pub translation_z: f64,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
    /// Optional variables that can be used to set velocity due to motion of the model. This can be
    /// used, for instance, if the motion of the models is estimated or measured externally, such as
    /// in an experiment, or when the model is used together with a rigid body simulator.
    pub motion_velocity_linear_x: f64,
    pub motion_velocity_linear_y: f64,
    pub motion_velocity_linear_z: f64,
    pub motion_velocity_angular_x: f64,
    pub motion_velocity_angular_y: f64,
    pub motion_velocity_angular_z: f64,
    /// Local wing angles for each wing. The angles are given in radians if `angles_in_degrees` is
    /// set to false, and in degrees if `angles_in_degrees` is set to true. It is the angle relative
    /// to the x-axis of the local body-fixed coordinate system.
    pub local_wing_angle_1: f64,
    pub local_wing_angle_2: f64,
    pub local_wing_angle_3: f64,
    pub local_wing_angle_4: f64,
    pub local_wing_angle_5: f64,
    pub local_wing_angle_6: f64,
    pub local_wing_angle_7: f64,
    pub local_wing_angle_8: f64,
    pub local_wing_angle_9: f64,
    pub local_wing_angle_10: f64,
    /// Internal state of the section models for each wing. The internal state is a single value
    /// that can be used to represent the state of the section model. The interpretation of the
    /// internal state depends on the sail type. For a wing sail, it can be the flap angle. For a
    /// rotor sail, it is the rotational speed.
    pub section_models_internal_state_1: f64,
    pub section_models_internal_state_2: f64,
    pub section_models_internal_state_3: f64,
    pub section_models_internal_state_4: f64,
    pub section_models_internal_state_5: f64,
    pub section_models_internal_state_6: f64,
    pub section_models_internal_state_7: f64,
    pub section_models_internal_state_8: f64,
    pub section_models_internal_state_9: f64,
    pub section_models_internal_state_10: f64,
    /// Optional variable to control the amount of thrust from controller
    pub controller_loading: f64,
    #[fmu_from_struct(output)]
    /// Error code. Zero means that everything is OK. Any other value means that an error has
    /// occurred, and that the model is not simulated anymore. All other outputs are then zero. See
    /// the README or the Stormbird book for the meaning of each code.
    pub error_code: i32,

    /// Global forces and moments acting on the lifting line model.
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,

    /// Individual sail forces
    pub force_sail_1_x: f64,
    pub force_sail_1_y: f64,
    pub force_sail_1_z: f64,
    pub moment_sail_1_x: f64,
    pub moment_sail_1_y: f64,
    pub moment_sail_1_z: f64,

    pub force_sail_2_x: f64,
    pub force_sail_2_y: f64,
    pub force_sail_2_z: f64,
    pub moment_sail_2_x: f64,
    pub moment_sail_2_y: f64,
    pub moment_sail_2_z: f64,

    pub force_sail_3_x: f64,
    pub force_sail_3_y: f64,
    pub force_sail_3_z: f64,
    pub moment_sail_3_x: f64,
    pub moment_sail_3_y: f64,
    pub moment_sail_3_z: f64,

    pub force_sail_4_x: f64,
    pub force_sail_4_y: f64,
    pub force_sail_4_z: f64,
    pub moment_sail_4_x: f64,
    pub moment_sail_4_y: f64,
    pub moment_sail_4_z: f64,

    pub force_sail_5_x: f64,
    pub force_sail_5_y: f64,
    pub force_sail_5_z: f64,
    pub moment_sail_5_x: f64,
    pub moment_sail_5_y: f64,
    pub moment_sail_5_z: f64,

    pub force_sail_6_x: f64,
    pub force_sail_6_y: f64,
    pub force_sail_6_z: f64,
    pub moment_sail_6_x: f64,
    pub moment_sail_6_y: f64,
    pub moment_sail_6_z: f64,

    pub force_sail_7_x: f64,
    pub force_sail_7_y: f64,
    pub force_sail_7_z: f64,
    pub moment_sail_7_x: f64,
    pub moment_sail_7_y: f64,
    pub moment_sail_7_z: f64,

    pub force_sail_8_x: f64,
    pub force_sail_8_y: f64,
    pub force_sail_8_z: f64,
    pub moment_sail_8_x: f64,
    pub moment_sail_8_y: f64,
    pub moment_sail_8_z: f64,

    pub force_sail_9_x: f64,
    pub force_sail_9_y: f64,
    pub force_sail_9_z: f64,
    pub moment_sail_9_x: f64,
    pub moment_sail_9_y: f64,
    pub moment_sail_9_z: f64,

    pub force_sail_10_x: f64,
    pub force_sail_10_y: f64,
    pub force_sail_10_z: f64,
    pub moment_sail_10_x: f64,
    pub moment_sail_10_y: f64,
    pub moment_sail_10_z: f64,

    /// Measurements of the effective angle of attack at different wings. Max 10 as output in the
    /// FMU
    pub angle_of_attack_measurement_1: f64,
    pub angle_of_attack_measurement_2: f64,
    pub angle_of_attack_measurement_3: f64,
    pub angle_of_attack_measurement_4: f64,
    pub angle_of_attack_measurement_5: f64,
    pub angle_of_attack_measurement_6: f64,
    pub angle_of_attack_measurement_7: f64,
    pub angle_of_attack_measurement_8: f64,
    pub angle_of_attack_measurement_9: f64,
    pub angle_of_attack_measurement_10: f64,
    /// Measurements of the wind velocity at different wings. Max 10 as output in the FMU
    pub velocity_measurement_1: f64,
    pub velocity_measurement_2: f64,
    pub velocity_measurement_3: f64,
    pub velocity_measurement_4: f64,
    pub velocity_measurement_5: f64,
    pub velocity_measurement_6: f64,
    pub velocity_measurement_7: f64,
    pub velocity_measurement_8: f64,
    pub velocity_measurement_9: f64,
    pub velocity_measurement_10: f64,
    /// Measurements of the apparent wind directions at different wings. Max 10 as output in the FMU
    pub apparent_wind_direction_measurement_1: f64,
    pub apparent_wind_direction_measurement_2: f64,
    pub apparent_wind_direction_measurement_3: f64,
    pub apparent_wind_direction_measurement_4: f64,
    pub apparent_wind_direction_measurement_5: f64,
    pub apparent_wind_direction_measurement_6: f64,
    pub apparent_wind_direction_measurement_7: f64,
    pub apparent_wind_direction_measurement_8: f64,
    pub apparent_wind_direction_measurement_9: f64,
    pub apparent_wind_direction_measurement_10: f64,

    /// Controller variables
    pub controller_section_models_internal_state_1: f64,
    pub controller_section_models_internal_state_2: f64,
    pub controller_section_models_internal_state_3: f64,
    pub controller_section_models_internal_state_4: f64,
    pub controller_section_models_internal_state_5: f64,
    pub controller_section_models_internal_state_6: f64,
    pub controller_section_models_internal_state_7: f64,
    pub controller_section_models_internal_state_8: f64,
    pub controller_section_models_internal_state_9: f64,
    pub controller_section_models_internal_state_10: f64,

    /// Calculated rigid body velocity, primarily used for debugging purposes.
    pub calculated_motion_velocity_linear_x: f64,
    pub calculated_motion_velocity_linear_y: f64,
    pub calculated_motion_velocity_linear_z: f64,
    pub calculated_motion_velocity_angular_x: f64,
    pub calculated_motion_velocity_angular_y: f64,
    pub calculated_motion_velocity_angular_z: f64,

    /// The FmuInfo variable is used by the fmu_from_struct macro to store information given about
    /// the FMU using the FMI-standard. This includes, for instance, the path to the unzipped
    /// resource directory which is later used to set a default path to the parameters file.
    pub fmu_info: FmuInfo,

    /// Tracker of number of calls to `do_step`
    iterations_completed: usize,
    /// Tracker of the number of time steps where the sail model was simulated
    model_steps_completed: usize,
    /// Whether the motion of the sails has been set from the input at least once
    motion_is_initialized: bool,

    /// Non public variables containing functionality from the Stormbird library
    parameters: Option<FmuParameters>,
    sail_model: Option<CompleteSailModel>,
    input_filters: Option<InputFilters>,
    time_model_scaling: Option<ModelScaling>
}

impl FmuFunctions for StormbirdLiftingLine {
    /// Sets up the model. Errors, including panics, are reported through the `error_code` output,
    /// rather than as an error status to the importer.
    fn exit_initialization_mode(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| self.initialize()));

        match result {
            Ok(Ok(())) => {},
            Ok(Err(error)) => self.set_error(error),
            Err(payload) => self.set_error(
                FmuError::from_panic(error_codes::PANIC_DURING_INITIALIZATION, payload)
            ),
        }
    }

    /// Simulates a time step. Errors, including panics, are reported through the `error_code`
    /// output, rather than as an error status to the importer. After an error, the model is not
    /// simulated anymore, and all the outputs are zero, until the FMU is restarted.
    fn do_step(&mut self, current_time: f64, time_step: f64) {
        if self.error_code != error_codes::OK {
            return;
        }

        let result = catch_unwind(AssertUnwindSafe(|| self.step(current_time, time_step)));

        match result {
            Ok(Ok(())) => {},
            Ok(Err(error)) => self.set_error(error),
            Err(payload) => self.set_error(
                FmuError::from_panic(error_codes::PANIC_DURING_TIME_STEP, payload)
            ),
        }
    }
}

impl StormbirdLiftingLine {
    /// Stores the error code as output, prints the error message, and removes the sail model, as
    /// its state may be inconsistent after an error. All other outputs are set to zero.
    fn set_error(&mut self, error: FmuError) {
        eprintln!("Stormbird lifting line FMU, {}", error);

        self.error_code = error.code;
        self.sail_model = None;

        self.set_zero_force_output();
        self.set_measurement_output(&[]);
    }

    /// Simulates a single time step
    fn step(&mut self, current_time_in: f64, time_step_in: f64) -> Result<(), FmuError> {
        let (current_time, time_step) = if let Some(scaling) = self.time_model_scaling {
            (scaling.upscale_time(current_time_in), scaling.upscale_time(time_step_in))
        } else {
            (current_time_in, time_step_in)
        };

        self.apply_filters_to_input_if_activated();

        let Some(parameters) = self.parameters.clone() else {
            return Err(FmuError::new(
                error_codes::MODEL_NOT_INITIALIZED,
                "The model is not set up. The initialization mode must be exited before simulating"
            ));
        };

        // The motion is updated in every time step, also while waiting for the simulation to start,
        // so that the velocities from finite differences are correct when the simulation starts.
        self.update_motion(&parameters, time_step)?;

        let waiting_iterations_is_done =
            self.iterations_completed >= parameters.number_of_iterations_before_building_model;

        self.iterations_completed += 1;

        if !waiting_iterations_is_done {
            return Ok(());
        }

        self.set_control_values(&parameters)?;

        let wind_condition = self.wind_condition(&parameters);
        let ship_velocity_vector = self.ship_velocity_vector(&parameters);

        let freestream_velocity = self.sail_model()?.freestream_velocity_with_ship_velocity_vector(
            &wind_condition,
            ship_velocity_vector,
            current_time
        );

        // The velocity felt by the sails, i.e., the freestream minus the motion of the sails. The
        // ship velocity can be part of either, depending on the settings.
        let felt_ctrl_points_velocity = {
            let line_force_model = &self.sail_model()?.lifting_line_simulation.line_force_model;

            line_force_model.felt_ctrl_points_velocity(
                &freestream_velocity[0..line_force_model.nr_span_lines()]
            )
        };

        let is_finite = |velocity: &SpatialVector| (0..3).all(|i| velocity[i].is_finite());

        let finite_input = freestream_velocity.iter().all(is_finite) &&
            felt_ctrl_points_velocity.iter().all(is_finite);

        if !finite_input {
            return Err(FmuError::new(
                error_codes::NON_FINITE_INPUT,
                "The freestream velocity or the motion velocity is not finite. Check the wind and \
                motion input"
            ));
        }

        let non_zero_input = felt_ctrl_points_velocity.iter()
            .any(|velocity| velocity.length_squared() > 0.0);

        if !non_zero_input {
            self.set_zero_force_output();

            return Ok(());
        }

        let controller_loading = self.controller_loading;

        let model = self.sail_model_mut()?;

        let result = model.lifting_line_simulation.do_step(
            current_time,
            time_step,
            &freestream_velocity
        );

        Self::check_result(&result)?;

        let flow_measurements = model.flow_measurements_from_simulation_result(
            controller_loading,
            &result
        );

        model.update_controller(current_time, time_step, &flow_measurements);

        self.model_steps_completed += 1;

        self.set_force_output(&result);
        self.set_measurement_output(&flow_measurements);

        Ok(())
    }

    fn sail_model(&self) -> Result<&CompleteSailModel, FmuError> {
        self.sail_model.as_ref().ok_or_else(|| FmuError::new(
            error_codes::MODEL_NOT_INITIALIZED,
            "The sail model is not set up"
        ))
    }

    fn sail_model_mut(&mut self) -> Result<&mut CompleteSailModel, FmuError> {
        self.sail_model.as_mut().ok_or_else(|| FmuError::new(
            error_codes::MODEL_NOT_INITIALIZED,
            "The sail model is not set up"
        ))
    }

    /// Checks that the forces and moments in the result are finite
    fn check_result(result: &SimulationResult) -> Result<(), FmuError> {
        let force = result.integrated_forces_sum();
        let moment = result.integrated_moments_sum();

        let is_finite = (0..3).all(|i| force[i].is_finite() && moment[i].is_finite());

        if is_finite {
            Ok(())
        } else {
            Err(FmuError::new(
                error_codes::NON_FINITE_RESULT,
                "The simulation gave forces or moments that are not finite"
            ))
        }
    }

    /// Sets the control values of the sails from the input, if there is no controller. With a
    /// controller, the input is only used as initial values.
    fn set_control_values(&mut self, parameters: &FmuParameters) -> Result<(), FmuError> {
        let nr_sails = self.sail_model()?.get_number_of_sails();

        let local_wing_angles = self.local_wing_angles(parameters, nr_sails);
        let section_models_internal_state = self.section_models_internal_state(nr_sails);

        let model_steps_completed = self.model_steps_completed;

        let model = self.sail_model_mut()?;

        if !model.has_controller() || model_steps_completed == 0 {
            model.set_local_wing_angles(&local_wing_angles);
            model.set_internal_state(&section_models_internal_state);
        }

        Ok(())
    }

    /// Updates the motion of the sails from the input.
    ///
    /// The translation and rotation are set from the input, and the velocities are computed with
    /// finite differences from the previous translation and rotation. In the first call, the
    /// translation and rotation are set directly, with zero velocity, as there is no previous
    /// motion. Otherwise, a non-zero initial position, e.g., a ship position far from the origin or
    /// a heading, would give a large velocity in the first time step. The velocities are replaced
    /// by the motion velocity input, if it is used.
    fn update_motion(&mut self, parameters: &FmuParameters, time_step: f64) -> Result<(), FmuError> {
        let translation = self.translation_vector();
        let rotation    = self.rotation_vector(parameters);

        let motion_velocity_linear  = self.motion_velocity_linear_vector(parameters);
        let motion_velocity_angular = self.motion_velocity_angular_vector(parameters);

        let motion_is_initialized = self.motion_is_initialized;

        let model = self.sail_model_mut()?;

        let line_force_model = &mut model.lifting_line_simulation.line_force_model;

        if motion_is_initialized {
            // Apply translation and rotation, and compute the velocity using finite differences
            line_force_model.set_translation_and_rotation_with_finite_difference_for_the_velocity(
                time_step,
                translation,
                rotation
            );
        } else {
            let motion = &mut line_force_model.rigid_body_motion;

            motion.translation = translation;
            motion.rotation = rotation;
            motion.velocity_linear = SpatialVector::default();
            motion.velocity_angular = SpatialVector::default();

            line_force_model.update_global_data_representations();
        }

        let calculated_velocity_linear = line_force_model.rigid_body_motion.velocity_linear;
        let calculated_velocity_angular = line_force_model.rigid_body_motion.velocity_angular;

        // Override the computed velocities if motion velocity is used
        if parameters.use_motion_velocity {
            line_force_model.rigid_body_motion.velocity_angular = motion_velocity_angular;

            // Only apply the linear velocity IF the linear motion velocity is NOT used as a
            // freestream condition. It does not make sense set these variables if the effect of
            // them is already included in the inflow velocity.
            if !parameters.use_motion_velocity_linear_as_freestream {
                line_force_model.rigid_body_motion.velocity_linear = motion_velocity_linear;
            }
        }

        // Set the computed velocity to the outputs
        self.calculated_motion_velocity_linear_x = calculated_velocity_linear[0];
        self.calculated_motion_velocity_linear_y = calculated_velocity_linear[1];
        self.calculated_motion_velocity_linear_z = calculated_velocity_linear[2];
        self.calculated_motion_velocity_angular_x = calculated_velocity_angular[0];
        self.calculated_motion_velocity_angular_y = calculated_velocity_angular[1];
        self.calculated_motion_velocity_angular_z = calculated_velocity_angular[2];

        self.motion_is_initialized = true;

        Ok(())
    }

    /// Function that checks if the filters are activated, and if yes, applies the filters to the
    /// input data.
    fn apply_filters_to_input_if_activated(&mut self) {
        if let Some(filters) = &mut self.input_filters {
            filters.translation_x.add(self.translation_x);
            filters.translation_y.add(self.translation_y);
            filters.translation_z.add(self.translation_z);

            filters.rotation_x.add(self.rotation_x);
            filters.rotation_y.add(self.rotation_y);
            filters.rotation_z.add(self.rotation_z);

            filters.motion_velocity_linear_x.add(self.motion_velocity_linear_x);
            filters.motion_velocity_linear_y.add(self.motion_velocity_linear_y);
            filters.motion_velocity_linear_z.add(self.motion_velocity_linear_z);

            filters.motion_velocity_angular_x.add(self.motion_velocity_angular_x);
            filters.motion_velocity_angular_y.add(self.motion_velocity_angular_y);
            filters.motion_velocity_angular_z.add(self.motion_velocity_angular_z);

            self.translation_x = filters.translation_x.get_average();
            self.translation_y = filters.translation_y.get_average();
            self.translation_z = filters.translation_z.get_average();

            self.rotation_x = filters.rotation_x.get_average();
            self.rotation_y = filters.rotation_y.get_average();
            self.rotation_z = filters.rotation_z.get_average();

            self.motion_velocity_linear_x = filters.motion_velocity_linear_x.get_average();
            self.motion_velocity_linear_y = filters.motion_velocity_linear_y.get_average();
            self.motion_velocity_linear_z = filters.motion_velocity_linear_z.get_average();

            self.motion_velocity_angular_x = filters.motion_velocity_angular_x.get_average();
            self.motion_velocity_angular_y = filters.motion_velocity_angular_y.get_average();
            self.motion_velocity_angular_z = filters.motion_velocity_angular_z.get_average();
        }
    }

    /// Converts an angle from the input to radians, if `angles_in_degrees` is set to true
    fn input_angle(parameters: &FmuParameters, angle: f64) -> f64 {
        if parameters.angles_in_degrees {
            angle.to_radians()
        } else {
            angle
        }
    }

    /// Converts an angle in radians to the output unit, i.e., degrees if `angles_in_degrees` is
    /// set to true
    fn output_angle(parameters: Option<&FmuParameters>, angle: f64) -> f64 {
        if parameters.is_some_and(|parameters| parameters.angles_in_degrees) {
            angle.to_degrees()
        } else {
            angle
        }
    }

    /// Returns the rotation as a vector, in radians
    fn rotation_vector(&self, parameters: &FmuParameters) -> SpatialVector {
        SpatialVector::new(
            Self::input_angle(parameters, self.rotation_x),
            Self::input_angle(parameters, self.rotation_y),
            Self::input_angle(parameters, self.rotation_z),
        )
    }

    /// Returns the translation as a vector
    fn translation_vector(&self) -> SpatialVector {
        SpatialVector::new(self.translation_x, self.translation_y, self.translation_z)
    }

    /// Returns the linear motion velocity as a vector, in the global coordinate system
    fn motion_velocity_linear_vector(&self, parameters: &FmuParameters) -> SpatialVector {
        let velocity = SpatialVector::new(
            self.motion_velocity_linear_x,
            self.motion_velocity_linear_y,
            self.motion_velocity_linear_z,
        );

        match &self.sail_model {
            Some(model) if parameters.motion_velocity_in_body_fixed_frame => {
                velocity.from_rotated_to_global_system(
                    self.rotation_vector(parameters),
                    model.lifting_line_simulation.line_force_model.rigid_body_motion.rotation_type
                )
            },
            _ => velocity,
        }
    }

    /// Returns the angular motion velocity as a vector in the global coordinate system, in radians
    /// per second. The input is the angular velocity vector, either in the global coordinate
    /// system, or in the body fixed coordinate system if `motion_velocity_in_body_fixed_frame` is
    /// true, e.g., (p, q, r) from a ship simulator. It is not the time derivative of the rotation
    /// angles.
    fn motion_velocity_angular_vector(&self, parameters: &FmuParameters) -> SpatialVector {
        let velocity = SpatialVector::new(
            Self::input_angle(parameters, self.motion_velocity_angular_x),
            Self::input_angle(parameters, self.motion_velocity_angular_y),
            Self::input_angle(parameters, self.motion_velocity_angular_z),
        );

        match &self.sail_model {
            Some(model) if parameters.motion_velocity_in_body_fixed_frame => {
                velocity.from_rotated_to_global_system(
                    self.rotation_vector(parameters),
                    model.lifting_line_simulation.line_force_model.rigid_body_motion.rotation_type
                )
            },
            _ => velocity,
        }
    }

    /// Returns the wind direction in radians, within +/- 180 degrees
    fn wind_direction(&self, parameters: &FmuParameters) -> f64 {
        let mut wind_direction = Self::input_angle(parameters, self.wind_direction_coming_from);

        if wind_direction < -PI {
            wind_direction += 2.0 * PI;
        } else if wind_direction > PI {
            wind_direction -= 2.0 * PI;
        }

        wind_direction
    }

    /// Returns the wind condition from the wind input, with a power model for the velocity
    /// variation with height
    fn wind_condition(&self, parameters: &FmuParameters) -> WindCondition {
        WindCondition {
            direction_coming_from: self.wind_direction(parameters),
            velocity_variation: VelocityVariation::PowerModel(
                PowerModel {
                    reference_velocity: self.wind_velocity,
                    reference_height: PowerModel::default_reference_height(),
                    power_factor: PowerModel::default_power_factor()
                }
            ),
            parallel_gust: None,
            perpendicular_gust: None,
            vertical_gust: None
        }
    }

    /// Returns the velocity of the ship that is used when computing the apparent wind. This is the
    /// linear motion velocity if `use_motion_velocity_linear_as_freestream` is true, and zero
    /// otherwise. Without it, the velocity of the ship is taken into account through the motion of
    /// the sails.
    fn ship_velocity_vector(&self, parameters: &FmuParameters) -> SpatialVector {
        if parameters.use_motion_velocity_linear_as_freestream {
            self.motion_velocity_linear_vector(parameters)
        } else {
            SpatialVector::default()
        }
    }

    /// Returns the local wing angles from the input, in radians, for the given number of sails
    fn local_wing_angles(&self, parameters: &FmuParameters, nr_sails: usize) -> Vec<f64> {
        let raw_local_wing_angles = [
            self.local_wing_angle_1,
            self.local_wing_angle_2,
            self.local_wing_angle_3,
            self.local_wing_angle_4,
            self.local_wing_angle_5,
            self.local_wing_angle_6,
            self.local_wing_angle_7,
            self.local_wing_angle_8,
            self.local_wing_angle_9,
            self.local_wing_angle_10,
        ];

        raw_local_wing_angles.iter()
            .take(nr_sails)
            .map(|angle| Self::input_angle(parameters, *angle))
            .collect()
    }

    /// Returns the internal state of the section models from the input, for the given number of
    /// sails
    fn section_models_internal_state(&self, nr_sails: usize) -> Vec<f64> {
        let raw_internal_state = [
            self.section_models_internal_state_1,
            self.section_models_internal_state_2,
            self.section_models_internal_state_3,
            self.section_models_internal_state_4,
            self.section_models_internal_state_5,
            self.section_models_internal_state_6,
            self.section_models_internal_state_7,
            self.section_models_internal_state_8,
            self.section_models_internal_state_9,
            self.section_models_internal_state_10,
        ];

        raw_internal_state.iter().take(nr_sails).copied().collect()
    }

    fn set_zero_force_output(&mut self) {
        self.set_force_and_moment_output(
            SpatialVector::default(),
            SpatialVector::default(),
            &[SpatialVector::default(); MAX_NR_SAILS],
            &[SpatialVector::default(); MAX_NR_SAILS],
        );
    }

    fn set_force_output(&mut self, result: &SimulationResult) {
        let mut sail_forces = [SpatialVector::default(); MAX_NR_SAILS];
        let mut sail_moments = [SpatialVector::default(); MAX_NR_SAILS];

        for i in 0..result.nr_of_wings().min(MAX_NR_SAILS) {
            sail_forces[i] = result.integrated_forces[i].total;
            sail_moments[i] = result.integrated_moments[i].total;
        }

        self.set_force_and_moment_output(
            result.integrated_forces_sum(),
            result.integrated_moments_sum(),
            &sail_forces,
            &sail_moments,
        );
    }

    /// Sets the force and moment outputs, both the total and for each sail
    fn set_force_and_moment_output(
        &mut self,
        force: SpatialVector,
        moment: SpatialVector,
        sail_forces: &[SpatialVector; MAX_NR_SAILS],
        sail_moments: &[SpatialVector; MAX_NR_SAILS],
    ) {
        self.force_x = force[0];
        self.force_y = force[1];
        self.force_z = force[2];

        self.moment_x = moment[0];
        self.moment_y = moment[1];
        self.moment_z = moment[2];

        let sail_outputs: [[&mut f64; 6]; MAX_NR_SAILS] = [
            [&mut self.force_sail_1_x, &mut self.force_sail_1_y, &mut self.force_sail_1_z,
             &mut self.moment_sail_1_x, &mut self.moment_sail_1_y, &mut self.moment_sail_1_z],
            [&mut self.force_sail_2_x, &mut self.force_sail_2_y, &mut self.force_sail_2_z,
             &mut self.moment_sail_2_x, &mut self.moment_sail_2_y, &mut self.moment_sail_2_z],
            [&mut self.force_sail_3_x, &mut self.force_sail_3_y, &mut self.force_sail_3_z,
             &mut self.moment_sail_3_x, &mut self.moment_sail_3_y, &mut self.moment_sail_3_z],
            [&mut self.force_sail_4_x, &mut self.force_sail_4_y, &mut self.force_sail_4_z,
             &mut self.moment_sail_4_x, &mut self.moment_sail_4_y, &mut self.moment_sail_4_z],
            [&mut self.force_sail_5_x, &mut self.force_sail_5_y, &mut self.force_sail_5_z,
             &mut self.moment_sail_5_x, &mut self.moment_sail_5_y, &mut self.moment_sail_5_z],
            [&mut self.force_sail_6_x, &mut self.force_sail_6_y, &mut self.force_sail_6_z,
             &mut self.moment_sail_6_x, &mut self.moment_sail_6_y, &mut self.moment_sail_6_z],
            [&mut self.force_sail_7_x, &mut self.force_sail_7_y, &mut self.force_sail_7_z,
             &mut self.moment_sail_7_x, &mut self.moment_sail_7_y, &mut self.moment_sail_7_z],
            [&mut self.force_sail_8_x, &mut self.force_sail_8_y, &mut self.force_sail_8_z,
             &mut self.moment_sail_8_x, &mut self.moment_sail_8_y, &mut self.moment_sail_8_z],
            [&mut self.force_sail_9_x, &mut self.force_sail_9_y, &mut self.force_sail_9_z,
             &mut self.moment_sail_9_x, &mut self.moment_sail_9_y, &mut self.moment_sail_9_z],
            [&mut self.force_sail_10_x, &mut self.force_sail_10_y, &mut self.force_sail_10_z,
             &mut self.moment_sail_10_x, &mut self.moment_sail_10_y, &mut self.moment_sail_10_z],
        ];

        for (i, outputs) in sail_outputs.into_iter().enumerate() {
            let values = [
                sail_forces[i][0], sail_forces[i][1], sail_forces[i][2],
                sail_moments[i][0], sail_moments[i][1], sail_moments[i][2],
            ];

            for (output, value) in outputs.into_iter().zip(values) {
                *output = value;
            }
        }
    }

    /// Sets the measured flow conditions and the current internal state for each sail as output.
    /// Sails without a measurement, including all sails if the slice is empty, get zero values.
    fn set_measurement_output(&mut self, flow_measurements: &[ControllerInput]) {
        let mut angles_of_attack = [0.0; MAX_NR_SAILS];
        let mut velocities = [0.0; MAX_NR_SAILS];
        let mut apparent_wind_directions = [0.0; MAX_NR_SAILS];
        let mut internal_states = [0.0; MAX_NR_SAILS];

        let parameters = self.parameters.as_ref();

        for (i, measurement) in flow_measurements.iter().take(MAX_NR_SAILS).enumerate() {
            angles_of_attack[i] = Self::output_angle(parameters, measurement.angle_of_attack);
            velocities[i] = measurement.velocity;
            apparent_wind_directions[i] = Self::output_angle(
                parameters,
                measurement.apparent_wind_direction
            );
            internal_states[i] = measurement.current_internal_state;
        }

        let outputs: [(&mut f64, &mut f64, &mut f64, &mut f64); MAX_NR_SAILS] = [
            (&mut self.angle_of_attack_measurement_1, &mut self.velocity_measurement_1,
             &mut self.apparent_wind_direction_measurement_1, &mut self.controller_section_models_internal_state_1),
            (&mut self.angle_of_attack_measurement_2, &mut self.velocity_measurement_2,
             &mut self.apparent_wind_direction_measurement_2, &mut self.controller_section_models_internal_state_2),
            (&mut self.angle_of_attack_measurement_3, &mut self.velocity_measurement_3,
             &mut self.apparent_wind_direction_measurement_3, &mut self.controller_section_models_internal_state_3),
            (&mut self.angle_of_attack_measurement_4, &mut self.velocity_measurement_4,
             &mut self.apparent_wind_direction_measurement_4, &mut self.controller_section_models_internal_state_4),
            (&mut self.angle_of_attack_measurement_5, &mut self.velocity_measurement_5,
             &mut self.apparent_wind_direction_measurement_5, &mut self.controller_section_models_internal_state_5),
            (&mut self.angle_of_attack_measurement_6, &mut self.velocity_measurement_6,
             &mut self.apparent_wind_direction_measurement_6, &mut self.controller_section_models_internal_state_6),
            (&mut self.angle_of_attack_measurement_7, &mut self.velocity_measurement_7,
             &mut self.apparent_wind_direction_measurement_7, &mut self.controller_section_models_internal_state_7),
            (&mut self.angle_of_attack_measurement_8, &mut self.velocity_measurement_8,
             &mut self.apparent_wind_direction_measurement_8, &mut self.controller_section_models_internal_state_8),
            (&mut self.angle_of_attack_measurement_9, &mut self.velocity_measurement_9,
             &mut self.apparent_wind_direction_measurement_9, &mut self.controller_section_models_internal_state_9),
            (&mut self.angle_of_attack_measurement_10, &mut self.velocity_measurement_10,
             &mut self.apparent_wind_direction_measurement_10, &mut self.controller_section_models_internal_state_10),
        ];

        for (i, (angle_of_attack, velocity, apparent_wind_direction, internal_state)) in
            outputs.into_iter().enumerate()
        {
            *angle_of_attack = angles_of_attack[i];
            *velocity = velocities[i];
            *apparent_wind_direction = apparent_wind_directions[i];
            *internal_state = internal_states[i];
        }
    }
}

#[cfg(test)]
mod tests;
