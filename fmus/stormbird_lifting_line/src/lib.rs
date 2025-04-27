// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

mod input_filters;
mod parameters;

use std::f64::consts::PI;
use std::path::PathBuf;

use fmu_from_struct::prelude::*;

use stormath::spatial_vector::SpatialVector;

use stormbird::empirical_models::wind_environment::height_variation::HeightVariationModel;
use stormbird::common_utils::result::SimulationResult;
use stormbird::lifting_line::simulation::Simulation;
use stormbird::lifting_line::simulation_builder::SimulationBuilder;

use fmu_from_struct::FmuInfo;

use input_filters::InputFilters;
use parameters::FmuParameters;

const WIND_ROTATION_AXIS: SpatialVector<3> = SpatialVector([0.0, 0.0, 1.0]);

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
/// FMU for a lifting line model using the Stormbird library.
pub struct StormbirdLiftingLine {
    #[parameter]
    /// Path to the parmaeters file. If empty, the parameters file is expected to be in the resource
    /// directory of the FMU.
    pub parameters_path: String,
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
    #[output]
    /// Global forces and moments acting on the lifting line model.
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,
    /// Estimation of the apparent wind direction, measured at the same local spanwise position as
    /// the angle of attack measurements.
    pub estimated_apparent_wind_direction: f64,
    /// Measurements of the effective angle of attack at the specified non-dimensional spanwise 
    /// location of each wing.
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

    /// The FmuInfo variable is used by the fmu_from_struct macro to store information given about
    /// the FMU using the FMI-standard. This includes, for instance, the path to the unzipped
    /// resource directory which is later used to set a default path to the parameters file.
    pub fmu_info: FmuInfo,

    parameters: FmuParameters,
    stormbird_model: Option<Simulation>,
    height_variation_model: Option<HeightVariationModel>,
    input_filters: Option<InputFilters>,
    iterations_completed: usize,
}

impl FmuFunctions for StormbirdLiftingLine {
    fn exit_initialization_mode(&mut self) {
        env_logger::init();

        self.read_parameters();
        self.build_wind_model();
        self.build_filters();
        self.build_lifting_line_model();
    }

    fn do_step(&mut self, current_time: f64, time_step: f64) {
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
                self.set_output(&result, &freestream_velocity);
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
                log::error!("Error reading lifting line setup file from path: {}. Error: {}", &setup_path.to_string_lossy(), e);
            }
        }
    }

    /// Builds filters for the input
    fn build_filters(&mut self) {
        if self.parameters.input_moving_average_window_size > 0 {
            log::info!("Building input filters with window size: {}", self.parameters.input_moving_average_window_size);

            self.input_filters = Some(
                InputFilters::new(self.parameters.input_moving_average_window_size)
            );
        }
    }

    /// Builds a wind environment model
    fn build_wind_model(&mut self) {
        log::info!("Building wind model");

        if !self.parameters.wind_environment_setup_file_path.is_empty() {
            let height_variation_model =
                HeightVariationModel::from_json_file(&self.parameters.wind_environment_setup_file_path);

            self.height_variation_model = Some(height_variation_model);
        }
    }

    /// Functions that sets the state of the line force model before a step is performed.
    fn set_line_force_model_state(&mut self, time_step: f64) {
        let translation = self.translation_vector();
        let rotation = self.rotation_vector();

        let motion_velocity_linear = self.motion_velocity_linear_vector();
        let motion_velocity_angular = self.motion_velocity_angular_vector();

        let local_wing_angles = self.local_wing_angles();
        let section_models_internal_state = self.section_models_internal_state();

        if let Some(model) = &mut self.stormbird_model {
            
            model.line_force_model.local_wing_angles = local_wing_angles;

            model.line_force_model
                .set_section_models_internal_state(&section_models_internal_state);

            if self.parameters.use_motion_velocity {
                model.line_force_model.rigid_body_motion.rotation = rotation;
                
                // Only apply translation and linear velocity IF the linear motion velocity is NOT
                // used as a freestream condition. It does not make sense set these variables if the 
                // effect of them is already included in the inflow velocity.
                if !self.parameters.use_motion_velocity_linear_as_freestream {
                    model.line_force_model.rigid_body_motion.translation = translation;
                    model.line_force_model.rigid_body_motion.velocity_linear = motion_velocity_linear;
                } 
                
                model.line_force_model.rigid_body_motion.velocity_angular = motion_velocity_angular;
            } else {
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
            SpatialVector([self.rotation_x, self.rotation_y, self.rotation_z])
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
        let wind_direction = self.wind_direction();

        let freestream_velocity_points: Vec<SpatialVector<3>> =
            if let Some(model) = &self.stormbird_model {
                model.get_freestream_velocity_points()
            } else {
                vec![]
            };

        let mut out = vec![SpatialVector([0.0, 0.0, 0.0]); freestream_velocity_points.len()];

        for i in 0..freestream_velocity_points.len() {
            let height = if self.parameters.negative_z_is_up {
                -freestream_velocity_points[i][2]
            } else {
                freestream_velocity_points[i][2]
            };

            let increase_factor = if let Some(model) = &self.height_variation_model {
                model.velocity_increase_factor(height)
            } else {
                1.0
            };

            out[i] = SpatialVector([
                -self.wind_velocity * increase_factor,
                0.0,
                0.0,
            ]).rotate_around_axis(wind_direction, WIND_ROTATION_AXIS);
        }

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

    pub fn section_models_internal_state(&self) -> Vec<f64> {
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

    /// Function that measures the apparent wind direction on each wing
    pub fn measure_apparent_wind_direction(&self, velocity: &[SpatialVector<3>]) -> Vec<f64> {
        let mut apparent_wind_direction = if let Some(model) = &self.stormbird_model {
            let felt_velocity = model.line_force_model.felt_ctrl_points_velocity(velocity);

            let relevant_velocities = model.line_force_model.interpolate_values_to_spanwise_location(
                self.parameters.non_dim_spanwise_measurement_position,
                &felt_velocity
            );

            let reference_vector = SpatialVector([-1.0, 0.0, 0.0]); // Negative x axis, as the angle is assumed to be 'coming from'

            let axis = if self.parameters.negative_z_is_up {
                SpatialVector([0.0, 0.0, -1.0])
            } else {
                SpatialVector([0.0, 0.0, 1.0])
            };

            relevant_velocities.iter().map(
                |velocity| {
                    reference_vector.signed_angle_between(*velocity, axis)
                }
            ).collect()
        } else {
            vec![0.0; self.nr_wings()]
        };

        if self.parameters.angles_in_degrees {
            for i in 0..apparent_wind_direction.len() {
                apparent_wind_direction[i] = apparent_wind_direction[i].to_degrees();
            }
        }

        apparent_wind_direction
    }

    /// Function that measures the angle of attack on each wing.
    fn measure_angles_of_attack(&self, angles_of_attack: &[f64]) -> Vec<f64> {
        let nr_wings = self.nr_wings();

        if nr_wings == 0 {
            return vec![];
        }

        let mut angles_of_attack = if let Some(model) = &self.stormbird_model {
            model.line_force_model.interpolate_values_to_spanwise_location(
                self.parameters.non_dim_spanwise_measurement_position,
                angles_of_attack
            )
        } else {
            vec![0.0; nr_wings]
        };

        if self.parameters.angles_in_degrees {
            for i in 0..angles_of_attack.len() {
                angles_of_attack[i] = angles_of_attack[i].to_degrees();
            }
        }

        angles_of_attack
    }

    fn set_output(&mut self, result: &SimulationResult, velocity_input: &[SpatialVector<3>]) {
        if let Some(model) = &self.stormbird_model {
            let velocity_input_at_line_force_models = &velocity_input[0..model.line_force_model.nr_span_lines()];

            let apparent_wind_directions = self.measure_apparent_wind_direction(
                velocity_input_at_line_force_models
            );

            self.estimated_apparent_wind_direction = apparent_wind_directions.iter()
                .sum::<f64>() / apparent_wind_directions.len() as f64;
        } else {
            self.estimated_apparent_wind_direction = 0.0;
        }
        
        let integrated_forces = result.integrated_forces_sum();
        let integrated_moments = result.integrated_moments_sum();

        self.force_x = integrated_forces[0];
        self.force_y = integrated_forces[1];
        self.force_z = integrated_forces[2];

        self.moment_x = integrated_moments[0];
        self.moment_y = integrated_moments[1];
        self.moment_z = integrated_moments[2];

        let angles_of_attack_measurement = self.measure_angles_of_attack(&result.force_input.angles_of_attack);

        let mut angle_of_attack_measurement_extended = vec![0.0; 10];

        for i in 0..angles_of_attack_measurement.len() {
            angle_of_attack_measurement_extended[i] = angles_of_attack_measurement[i];
        }

        self.angle_of_attack_measurement_1  = angle_of_attack_measurement_extended[0];
        self.angle_of_attack_measurement_2  = angle_of_attack_measurement_extended[1];
        self.angle_of_attack_measurement_3  = angle_of_attack_measurement_extended[2];
        self.angle_of_attack_measurement_4  = angle_of_attack_measurement_extended[3];
        self.angle_of_attack_measurement_5  = angle_of_attack_measurement_extended[4];
        self.angle_of_attack_measurement_6  = angle_of_attack_measurement_extended[5];
        self.angle_of_attack_measurement_7  = angle_of_attack_measurement_extended[6];
        self.angle_of_attack_measurement_8  = angle_of_attack_measurement_extended[7];
        self.angle_of_attack_measurement_9  = angle_of_attack_measurement_extended[8];
        self.angle_of_attack_measurement_10 = angle_of_attack_measurement_extended[9];
    }
}
