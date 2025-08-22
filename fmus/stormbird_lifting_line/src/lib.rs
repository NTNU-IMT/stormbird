// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

mod input_filters;
mod parameters;
mod model_scaling;

use std::f64::consts::PI;
use std::path::PathBuf;

use fmu_from_struct::prelude::*;

use stormath::spatial_vector::SpatialVector;


use stormbird::common_utils::results::simulation::SimulationResult;
use stormbird::lifting_line::simulation::Simulation;
use stormbird::lifting_line::simulation_builder::SimulationBuilder;

use stormbird::wind::environment::{
    WindEnvironment,
    WindCondition
};

use stormbird::controllers::{
    Controller,
    ControllerBuilder,
    input::ControllerInput,
    output::ControllerOutput,
    measurements::FlowMeasurementSettings
};

use fmu_from_struct::FmuInfo;

use input_filters::InputFilters;
use parameters::FmuParameters;
use model_scaling::ModelScaling;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
/// FMU for a lifting line model using the Stormbird library.
pub struct StormbirdLiftingLine {
    #[parameter]
    /// Path to the parameters file. If empty, the parameters file is expected to be in the resource
    /// directory of the FMU.
    pub parameters_path: String,
    pub time_model_scale: f64,
    #[input]
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
    #[output]
    /// Global forces and moments acting on the lifting line model.
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,
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

    /// Tracker of number of iterations completed
    iterations_completed: usize,

    /// Non public variables containing functionality from the Stormbird library
    parameters: FmuParameters,
    stormbird_model: Option<Simulation>,
    wind_environment: Option<WindEnvironment>,
    controller: Option<Controller>,
    input_filters: Option<InputFilters>,
    time_model_scaling: Option<ModelScaling>
}

impl FmuFunctions for StormbirdLiftingLine {
    fn exit_initialization_mode(&mut self) {
        env_logger::init();

        self.read_parameters();
        self.build_wind_model();
        self.build_controller();
        self.build_filters();
        self.build_lifting_line_model();

        if self.time_model_scale > 0.0 {
            self.time_model_scaling = Some(
                ModelScaling{
                    scale: self.time_model_scale
                }
            );
        }
    }

    fn do_step(&mut self, current_time_in: f64, time_step_in: f64) {
        let (current_time, time_step) = if let Some(scaling) = self.time_model_scaling {
            (scaling.upscale_time(current_time_in), scaling.upscale_time(time_step_in))
        } else {
            (current_time_in, time_step_in)
        };
        
        self.apply_filters_to_input_if_activated();

        let waiting_iterations_is_done = 
            self.iterations_completed >= self.parameters.number_of_iterations_before_building_model;

        if self.stormbird_model.is_some() && waiting_iterations_is_done {
            self.set_line_force_model_state(time_step);

            let freestream_velocity = self.freestream_velocity();

            let result = if let Some(model) = &mut self.stormbird_model {
                Some(
                    model.do_step(current_time, time_step, &freestream_velocity)
                )
            } else {
                None
            };

            if let Some(result) = result {
                let controller_input = self.controller_input(&result);

                self.set_force_output(&result);

                self.set_controller_measurement_output(&controller_input);

                self.apply_controller(current_time, time_step, &controller_input)
            }
        }

        self.iterations_completed += 1;
    }
}

impl StormbirdLiftingLine {
    /// Function that checks if the parameters file path is set, and if not, sets the default path
    /// to the resource directory of the FMU. 
    fn parameters_path(&self) -> PathBuf {
        let parameters_path: PathBuf = if self.parameters_path.is_empty() {
            let mut path = self.fmu_info.resource_path.clone();

            path.push("parameters.json");

            path
        } else {
            PathBuf::from(&self.parameters_path)
        };

        log::info!("Parameters path: {:?}", parameters_path);

        parameters_path
    }

    /// Function that reads the parameters from the parameters file.
    fn read_parameters(&mut self) {
        log::info!("Reading parameters from file");
        let parameters_path = self.parameters_path();

        let parameters = FmuParameters::from_json_file(&parameters_path);

        match parameters {
            Ok(parameters) => {
                self.parameters = parameters;
            },
            Err(e) => {
                log::error!("Error reading parameters file: {}", e);
            }
        }
    }

    /// Builds the sail model using the lifting liner setup file
    fn build_lifting_line_model(&mut self) {
        log::info!("Building lifting line model for the sails");

        let mut setup_path = self.parameters_path();
        setup_path.pop();
        setup_path.push(self.parameters.lifting_line_setup_file_path.clone());

        let stormbird_model_builder =
            SimulationBuilder::new_from_file(&setup_path.to_string_lossy());

        match stormbird_model_builder {
            Ok(builder) => {
                self.stormbird_model = Some(builder.build());
            },
            Err(e) => {
                log::error!(
                    "Error reading lifting line setup file from path: {}. Error: {}", 
                    &setup_path.to_string_lossy(), 
                    e
                );
            }
        }
    }

    /// Builds filters for the input
    fn build_filters(&mut self) {
        if self.parameters.input_moving_average_window_size > 0 {
            log::info!(
                "Building input filters with window size: {}", 
                self.parameters.input_moving_average_window_size
            );

            self.input_filters = Some(
                InputFilters::new(self.parameters.input_moving_average_window_size)
            );
        }
    }

    /// Builds a wind environment model
    fn build_wind_model(&mut self) {
        log::info!("Building wind model");

        if !self.parameters.wind_environment_setup_file_path.is_empty() {
            let mut setup_path = self.parameters_path();
            setup_path.pop();
            setup_path.push(self.parameters.wind_environment_setup_file_path.clone());

            let environment = WindEnvironment::from_json_file(&setup_path.to_string_lossy());

            match environment {
                Ok(env) => {
                    self.wind_environment = Some(env);
                },
                Err(e) => {
                    log::error!(
                        "Error reading wind environment setup file from path: {}. Error: {}", 
                        &self.parameters.wind_environment_setup_file_path, 
                        e
                    );
                }
            }
        } else {
            self.wind_environment = Some(WindEnvironment::default());
        }
    }

    fn build_controller(&mut self) {
        log::info!("Building controller");

        if !self.parameters.controller_setup_file_path.is_empty() {
            let mut setup_path = self.parameters_path();
            setup_path.pop();
            setup_path.push(self.parameters.controller_setup_file_path.clone());
            
            let controller_builder = ControllerBuilder::from_json_file(
                &setup_path.to_string_lossy()
            );

            match controller_builder {
                Ok(builder) => {
                    self.controller = Some(builder.build());
                },
                Err(e) => {
                    log::error!(
                        "Error reading controller setup file from path: {}. Error: {}", 
                        &self.parameters.controller_setup_file_path, 
                        e
                    );
                }
            }
        } else {
            log::warn!("No controller setup file path provided, skipping controller setup.");
        }
    }

    fn apply_controller(
        &mut self,
        current_time: f64, 
        time_step: f64, 
        controller_input: &ControllerInput
    ) {
        if let Some(controller) = &self.controller {
            let controller_output = controller.update(
                current_time,
                time_step,
                &controller_input
            );
            
            if let Some(output) = &controller_output {
                self.set_model_control_values_from_controller_output(output);
            } 
        }
    }

    fn set_model_control_values_from_input(&mut self) {
        let local_wing_angles = self.local_wing_angles();
        let section_models_internal_state = self.section_models_internal_state();

        if let Some(model) = &mut self.stormbird_model {
            
            model.line_force_model.local_wing_angles = local_wing_angles;

            model.line_force_model
                .set_section_models_internal_state(&section_models_internal_state);
        }
    }

    fn set_model_control_values_from_controller_output(&mut self, controller_output: &ControllerOutput) {
        if let Some(model) = &mut self.stormbird_model {
            model.line_force_model.set_controller_output(controller_output)
        }
    }

    /// Functions that sets the state of the line force model before a step is performed.
    fn set_line_force_model_state(&mut self, time_step: f64) {
        let translation = self.translation_vector();
        let rotation    = self.rotation_vector();

        let motion_velocity_linear  = self.motion_velocity_linear_vector();
        let motion_velocity_angular = self.motion_velocity_angular_vector();

        if self.controller.is_none() || self.iterations_completed == 0 {
            self.set_model_control_values_from_input()
        }

        if let Some(model) = &mut self.stormbird_model {
            model
                .line_force_model
                .rigid_body_motion
                .update_translation_with_velocity_using_finite_difference(
                    translation,
                    time_step
                );

            model
                .line_force_model
                .rigid_body_motion
                .update_rotation_with_velocity_using_finite_difference(
                    rotation,
                    time_step
                );

            self.calculated_motion_velocity_linear_x = model.line_force_model.rigid_body_motion.velocity_linear[0];
            self.calculated_motion_velocity_linear_y = model.line_force_model.rigid_body_motion.velocity_linear[1];
            self.calculated_motion_velocity_linear_z = model.line_force_model.rigid_body_motion.velocity_linear[2];
            self.calculated_motion_velocity_angular_x = model.line_force_model.rigid_body_motion.velocity_angular[0];
            self.calculated_motion_velocity_angular_y = model.line_force_model.rigid_body_motion.velocity_angular[1];
            self.calculated_motion_velocity_angular_z = model.line_force_model.rigid_body_motion.velocity_angular[2];

            if self.parameters.use_motion_velocity {
                model.line_force_model.rigid_body_motion.velocity_angular = motion_velocity_angular;
                
                // Only apply the linear velocity IF the linear motion velocity is NOT used as a 
                // freestream condition. It does not make sense set these variables if the effect of 
                // them is already included in the inflow velocity.
                if !self.parameters.use_motion_velocity_linear_as_freestream {
                    model.line_force_model.rigid_body_motion.velocity_linear = motion_velocity_linear;
                }
            }
        }
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

    fn nr_wings(&self) -> usize {
        if let Some(model) = &self.stormbird_model {
            model.line_force_model.nr_wings()
        } else {
            0
        }
    }

    /// Returns the rotation as a vector. If `angles_in_degrees` is set to true, the angles are 
    /// converted to radians.
    fn rotation_vector(&self) -> SpatialVector<3> {
        if self.parameters.angles_in_degrees {
            SpatialVector([
                self.rotation_x.to_radians(),
                self.rotation_y.to_radians(),
                self.rotation_z.to_radians(),
            ])
        } else {
            SpatialVector([
                self.rotation_x, 
                self.rotation_y, 
                self.rotation_z
            ])
        }
    }

    /// Returns the translation as a vector
    fn translation_vector(&self) -> SpatialVector<3> {
        SpatialVector([self.translation_x, self.translation_y, self.translation_z])
    }

    /// Returns the linear motion velocity as a vector
    fn motion_velocity_linear_vector(&self) -> SpatialVector<3> {
        let mut out = SpatialVector([
            self.motion_velocity_linear_x,
            self.motion_velocity_linear_y,
            self.motion_velocity_linear_z,
        ]);
        
        if let Some(model) = &self.stormbird_model {
            if self.parameters.motion_velocity_in_body_fixed_frame {
                let rotation = self.rotation_vector();

                out = out.from_rotated_to_global_system(
                    rotation,
                    model.line_force_model.rigid_body_motion.rotation_type
                );
            }
        }

        out
    }

    /// Returns the angular motion velocity as a vector
    fn motion_velocity_angular_vector(&self) -> SpatialVector<3> {
        if self.parameters.angles_in_degrees {
            SpatialVector([
                self.motion_velocity_angular_x.to_radians(),
                self.motion_velocity_angular_y.to_radians(),
                self.motion_velocity_angular_z.to_radians(),
            ])
        } else {
            SpatialVector([
                self.motion_velocity_angular_x, 
                self.motion_velocity_angular_y, 
                self.motion_velocity_angular_z
            ])
        }
    }

    fn wind_direction(&self) -> f64 {
        let mut wind_direction = if self.parameters.angles_in_degrees {
            self.wind_direction_coming_from.to_radians()
        } else {
            self.wind_direction_coming_from
        };

        if self.parameters.reverse_wind_direction {
            wind_direction *= -1.0;
        }

        // Ensure wind direction is within +/- 180 degrees
        if wind_direction < -PI {
            wind_direction += 2.0 * PI;
        } else if wind_direction > PI {
            wind_direction -= 2.0 * PI;
        }

        wind_direction
    }

    /// Function that returns the velocity inflow to the lifting line model. The function combines
    /// the wind velocity and the translational velocity of the model.
    fn freestream_velocity(&self) -> Vec<SpatialVector<3>> {
        // Collect the relevant points to calculate the wind condition for
        let freestream_velocity_points: Vec<SpatialVector<3>> =
            if let Some(model) = &self.stormbird_model {
                model.get_freestream_velocity_points()
            } else {
                vec![]
            };
        
        // Get the wind field from the wind environment, based on the wind condition
        let wind_condition = WindCondition {
            reference_velocity: self.wind_velocity,
            direction_coming_from: self.wind_direction()
        };

        let mut out = if let Some(env) = &self.wind_environment {
            env.true_wind_velocity_vectors_at_locations(wind_condition, &freestream_velocity_points)
        } else {
            panic!("Wind environment is not defined!")
        };

        // Apply the linear motion of the wings to the freestream if this option is activated
        if self.parameters.use_motion_velocity_linear_as_freestream {
            let motion_velocity = self.motion_velocity_linear_vector();

            for i in 0..out.len() {
                out[i] -= motion_velocity;
            }
        }

       out
    }

    fn local_wing_angles(&self) -> Vec<f64> {
        let nr_wings = self.nr_wings();
        
        if nr_wings == 0 {
            return vec![];
        }

        let mut local_wing_angles: Vec<f64> = vec![0.0; nr_wings];

        let raw_local_wing_angles = vec![
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

        for i in 0..nr_wings {
            local_wing_angles[i] = raw_local_wing_angles[i];
        }

        if self.parameters.angles_in_degrees {
            for i in 0..local_wing_angles.len() {
                local_wing_angles[i] = local_wing_angles[i].to_radians();
            }
        }

        local_wing_angles
    }

    fn section_models_internal_state(&self) -> Vec<f64> {
        let nr_wings = self.nr_wings();

        if nr_wings == 0 {
            return vec![];
        }

        let section_models_internal_state_raw = vec![
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

        let mut section_models_internal_state = vec![0.0; nr_wings];

        for i in 0..nr_wings {
            section_models_internal_state[i] = section_models_internal_state_raw[i];
        }
        
        section_models_internal_state
    }

    fn set_force_output(&mut self, result: &SimulationResult) {
        let integrated_forces = result.integrated_forces_sum();
        let integrated_moments = result.integrated_moments_sum();

        self.force_x = integrated_forces[0];
        self.force_y = integrated_forces[1];
        self.force_z = integrated_forces[2];

        self.moment_x = integrated_moments[0];
        self.moment_y = integrated_moments[1];
        self.moment_z = integrated_moments[2];
    }

    fn controller_input(&self, result: &SimulationResult) -> ControllerInput {
        match (&self.stormbird_model, &self.wind_environment, &self.controller) {
            (Some(model), Some(environment), Some(controller)) => {
                return ControllerInput::new(
                    self.controller_loading,
                    &model.line_force_model,
                    result,
                    &controller.flow_measurement_settings,
                    environment
                )
            },
            (Some(model), Some(environment), None) => {
                return ControllerInput::new(
                    self.controller_loading,
                    &model.line_force_model,
                    result,
                    &FlowMeasurementSettings::default(),
                    environment
                )
            },
            _ => {
                panic!("Missing either lifting line model or wind environment")
            }
        }
    }

    /// Takes a ControllerInput variable as input, an applies the data to the output variables in 
    /// the FMU
    fn set_controller_measurement_output(&mut self, controller_input: &ControllerInput) {
        let output_size = 10;

        let mut angles_of_attack_extended = vec![0.0; output_size];
        let mut velocity_extended = vec![0.0; output_size];
        let mut apparent_wind_directions_extended = vec![0.0; output_size];
        let mut section_models_internal_state = vec![0.0; output_size];
        
        let nr_wings = self.nr_wings();

        for i in 0..nr_wings {
            velocity_extended[i] = controller_input.velocity[i];
            section_models_internal_state[i] = controller_input.current_section_models_internal_state[i];

            if self.parameters.angles_in_degrees {
                angles_of_attack_extended[i] = controller_input.angles_of_attack[i].to_degrees();
                apparent_wind_directions_extended[i] = controller_input.apparent_wind_directions[i].to_degrees();
            } else {
                angles_of_attack_extended[i] = controller_input.angles_of_attack[i];
                apparent_wind_directions_extended[i] = controller_input.apparent_wind_directions[i];
            }
            
        }

        self.angle_of_attack_measurement_1  = angles_of_attack_extended[0];
        self.angle_of_attack_measurement_2  = angles_of_attack_extended[1];
        self.angle_of_attack_measurement_3  = angles_of_attack_extended[2];
        self.angle_of_attack_measurement_4  = angles_of_attack_extended[3];
        self.angle_of_attack_measurement_5  = angles_of_attack_extended[4];
        self.angle_of_attack_measurement_6  = angles_of_attack_extended[5];
        self.angle_of_attack_measurement_7  = angles_of_attack_extended[6];
        self.angle_of_attack_measurement_8  = angles_of_attack_extended[7];
        self.angle_of_attack_measurement_9  = angles_of_attack_extended[8];
        self.angle_of_attack_measurement_10 = angles_of_attack_extended[9];

        self.velocity_measurement_1  = velocity_extended[0];
        self.velocity_measurement_2  = velocity_extended[1];
        self.velocity_measurement_3  = velocity_extended[2];
        self.velocity_measurement_4  = velocity_extended[3];
        self.velocity_measurement_5  = velocity_extended[4];
        self.velocity_measurement_6  = velocity_extended[5];
        self.velocity_measurement_7  = velocity_extended[6];
        self.velocity_measurement_8  = velocity_extended[7];
        self.velocity_measurement_9  = velocity_extended[8];
        self.velocity_measurement_10 = velocity_extended[9];

        self.apparent_wind_direction_measurement_1  = apparent_wind_directions_extended[0];
        self.apparent_wind_direction_measurement_2  = apparent_wind_directions_extended[1];
        self.apparent_wind_direction_measurement_3  = apparent_wind_directions_extended[2];
        self.apparent_wind_direction_measurement_4  = apparent_wind_directions_extended[3];
        self.apparent_wind_direction_measurement_5  = apparent_wind_directions_extended[4];
        self.apparent_wind_direction_measurement_6  = apparent_wind_directions_extended[5];
        self.apparent_wind_direction_measurement_7  = apparent_wind_directions_extended[6];
        self.apparent_wind_direction_measurement_8  = apparent_wind_directions_extended[7];
        self.apparent_wind_direction_measurement_9  = apparent_wind_directions_extended[8];
        self.apparent_wind_direction_measurement_10 = apparent_wind_directions_extended[9];

        self.controller_section_models_internal_state_1  = section_models_internal_state[0];
        self.controller_section_models_internal_state_2  = section_models_internal_state[1];
        self.controller_section_models_internal_state_3  = section_models_internal_state[2];
        self.controller_section_models_internal_state_4  = section_models_internal_state[3];
        self.controller_section_models_internal_state_5  = section_models_internal_state[4];
        self.controller_section_models_internal_state_6  = section_models_internal_state[5];
        self.controller_section_models_internal_state_7  = section_models_internal_state[6];
        self.controller_section_models_internal_state_8  = section_models_internal_state[7];
        self.controller_section_models_internal_state_9  = section_models_internal_state[8];
        self.controller_section_models_internal_state_10 = section_models_internal_state[9];

    }
}
