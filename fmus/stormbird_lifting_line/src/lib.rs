// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

mod model_scaling;

use std::f64::consts::PI;

use fmu_from_struct::prelude::*;

use math_utils::{
    spatial_vector::SpatialVector,
    filters::moving_average::MovingAverage,
};

use stormbird::empirical_models::wind_environment::height_variation::HeightVariationModel;
use stormbird::io_structs::result::SimulationResult;
use stormbird::lifting_line::simulation::Simulation;
use stormbird::lifting_line::simulation_builder::SimulationBuilder;
use stormbird::error::Error;

use serde::{Deserialize, Serialize};
use serde_json;

use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

fn print_to_file(s: &str, file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    file.write_all(s.as_bytes()).unwrap();
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Parameters {
    pub lifting_line_setup_file_path: String,
    #[serde(default)]
    pub wind_environment_setup_file_path: String,
    #[serde(default)]
    pub angles_in_degrees: bool,
    #[serde(default)]
    pub negative_z_is_up: bool,
    #[serde(default)]
    pub reverse_wind_direction: bool,
    #[serde(default)]
    pub reverse_translational_velocity: bool,
    #[serde(default)]
    pub non_dim_spanwise_measurement_position: f64,
    #[serde(default)]
    pub visualization_server_address: String,
    #[serde(default)]
    pub model_scale_factor: f64,
    #[serde(default)]
    pub input_moving_average_window_size: usize,
    #[serde(default)]
    pub max_input_velocity: Option<f64>,
    #[serde(default)]
    pub translational_velocity_in_body_fixed_frame: bool,
    #[serde(default)]
    pub max_position_change_velocity: Option<f64>,
    #[serde(default)]
    pub max_rotation_change_velocity: Option<f64>,
}

impl Parameters {
    fn from_json_file(file_path: &str) -> Result<Self, Error> {
        let file = File::open(file_path)?;

        let reader = std::io::BufReader::new(file);
        let result = serde_json::from_reader(reader)?;

        Ok(result)
    }
}

#[derive(Debug, Default, Clone)]
pub struct InputFilters {
    pub x_position: MovingAverage,
    pub y_position: MovingAverage,
    pub z_position: MovingAverage,
    pub x_rotation: MovingAverage,
    pub y_rotation: MovingAverage,
    pub z_rotation: MovingAverage,
    pub translational_velocity_x: MovingAverage,
    pub translational_velocity_y: MovingAverage,
    pub translational_velocity_z: MovingAverage,
}

impl InputFilters {
    pub fn new(window_size: usize) -> Self {
        Self {
            x_position: MovingAverage::new(window_size),
            y_position: MovingAverage::new(window_size),
            z_position: MovingAverage::new(window_size),
            x_rotation: MovingAverage::new(window_size),
            y_rotation: MovingAverage::new(window_size),
            z_rotation: MovingAverage::new(window_size),
            translational_velocity_x: MovingAverage::new(window_size),
            translational_velocity_y: MovingAverage::new(window_size),
            translational_velocity_z: MovingAverage::new(window_size),
        }
    }
}

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
/// FMU for a lifting line model using the Stormbird library.
pub struct StormbirdLiftingLine {
    #[parameter]
    pub parameters_path: String,
    #[input]
    pub translational_velocity_x: f64,
    pub translational_velocity_y: f64,
    pub translational_velocity_z: f64,
    pub wind_velocity: f64,
    pub wind_direction_coming_from: f64,
    pub x_position: f64,
    pub y_position: f64,
    pub z_position: f64,
    pub x_rotation: f64,
    pub y_rotation: f64,
    pub z_rotation: f64,
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

    parameters: Parameters,
    stormbird_model: Option<Simulation>,
    height_variation_model: Option<HeightVariationModel>,
    nr_wings: usize,
    nr_freestream_velocity_points: usize,
    initialized_wake_points: bool,
    input_filters: Option<InputFilters>,
    visualization_client: Option<reqwest::blocking::Client>,
    previous_position: SpatialVector<3>,
    previous_rotation: SpatialVector<3>,
}

impl FmuFunctions for StormbirdLiftingLine {
    fn exit_initialization_mode(&mut self) {
        let parameters_path = if self.parameters_path.is_empty() {
            "C:/HLCC 2024 x64/DLL_FMU's/Stormbird/stormbird_parameters.json".to_string() // Default string to facilitate using this FMU in hybrid tests.
        } else {
            self.parameters_path.clone()
        };

        let parameters = Parameters::from_json_file(&parameters_path);

        match parameters {
            Ok(parameters) => {
                self.parameters = parameters;
            },
            Err(e) => {
                let error_string = format!("Error reading parameters file: {}", e);

                print_to_file(&error_string, "parameters_error.txt");
            }
        }

        let initial_wake_builder_velocity = SpatialVector([1e-6, 0.0, 0.0]);

        let stormbird_model_builder =
            SimulationBuilder::new_from_file(&self.parameters.lifting_line_setup_file_path);

        match stormbird_model_builder {
            Ok(builder) => {
                self.stormbird_model = Some(builder.build(1.0, initial_wake_builder_velocity));
            },
            Err(e) => {
                let error_string = format!("Error reading lifting line setup file: {}", e);

                print_to_file(&error_string, "builder_error.txt");
            }
        }

        if let Some(model) = &mut self.stormbird_model {
            let freestream_velocity_points = model.get_freestream_velocity_points();

            self.nr_freestream_velocity_points = freestream_velocity_points.len();
            self.nr_wings = model.line_force_model.nr_wings();
        }

        if !self.parameters.wind_environment_setup_file_path.is_empty() {
            let height_variation_model =
                HeightVariationModel::from_json_file(&self.parameters.wind_environment_setup_file_path);

            self.height_variation_model = Some(height_variation_model);
        }

        if !self.parameters.visualization_server_address.is_empty() {
            self.visualization_client = Some(reqwest::blocking::Client::new());
        }

        if self.parameters.input_moving_average_window_size > 0 {
            self.input_filters = Some(InputFilters::new(self.parameters.input_moving_average_window_size));
        }
    }

    fn do_step(&mut self, current_time_in: f64, time_step_in: f64) {
        self.process_input();

        if self.zero_velocity() {
            return;
        }

        let current_time = self.upscaled_time_value(current_time_in);
        let time_step = self.upscaled_time_value(time_step_in);

        let rotation    = self.rotation(time_step);
        let translation = self.translation(time_step);

        let velocity_input: Vec<SpatialVector<3>> = self.velocity_input(time_step);

        let local_wing_angles = self.local_wing_angles();
        let section_models_internal_state = self.section_models_internal_state();

        let result = if let Some(model) = &mut self.stormbird_model {            
            model.line_force_model.rotation = rotation;
            model.line_force_model.translation = translation;
            model.line_force_model.local_wing_angles = local_wing_angles;

            model.line_force_model
                .set_section_models_internal_state(&section_models_internal_state);

            if !self.initialized_wake_points {
                let average_velocity =
                    velocity_input.iter().sum::<SpatialVector<3>>() / velocity_input.len() as f64;

                model.wake.initialize(&model.line_force_model, average_velocity, time_step);

                self.initialized_wake_points = true;
            }

            let result = model.do_step(current_time, time_step, &velocity_input);

            Some(result)
        } else {
            None
        };

        self.post_do_step(time_step);

        if let Some(result) = result {
            self.set_output(&result, &velocity_input, time_step);

            if self.visualization_client.is_some() {
                self.send_result_to_visualization_server(&result, current_time_in);
            }
        }
    }
}

impl StormbirdLiftingLine {
    fn process_input(&mut self) {
        if let Some(filters) = &mut self.input_filters {
            filters.x_position.add(self.x_position);
            filters.y_position.add(self.y_position);
            filters.z_position.add(self.z_position);
            filters.x_rotation.add(self.x_rotation);
            filters.y_rotation.add(self.y_rotation);
            filters.z_rotation.add(self.z_rotation);
            filters.translational_velocity_x.add(self.translational_velocity_x);
            filters.translational_velocity_y.add(self.translational_velocity_y);
            filters.translational_velocity_z.add(self.translational_velocity_z);

            self.x_position = filters.x_position.get_average();
            self.y_position = filters.y_position.get_average();
            self.z_position = filters.z_position.get_average();
            self.x_rotation = filters.x_rotation.get_average();
            self.y_rotation = filters.y_rotation.get_average();
            self.z_rotation = filters.z_rotation.get_average();
            self.translational_velocity_x = filters.translational_velocity_x.get_average();
            self.translational_velocity_y = filters.translational_velocity_y.get_average();
            self.translational_velocity_z = filters.translational_velocity_z.get_average();
        }
    }

    pub fn post_do_step(&mut self, time_step: f64) {
        let translation = self.translation(time_step);
        let rotation = self.rotation(time_step);

        self.previous_position = translation;
        self.previous_rotation = rotation;
    }

    fn rotation(&self, time_step: f64) -> SpatialVector<3> {
        let rotation_raw = if self.parameters.angles_in_degrees {
            SpatialVector([
                self.x_rotation.to_radians(),
                self.y_rotation.to_radians(),
                self.z_rotation.to_radians(),
            ])
        } else {
            SpatialVector([self.x_rotation, self.y_rotation, self.z_rotation])
        };

        let rotation_change_velocity = (rotation_raw - self.previous_rotation) / time_step;
        
        let rotation_change_velocity_magnitude = rotation_change_velocity.length();

        if let Some(max_value) = self.parameters.max_rotation_change_velocity {
            if rotation_change_velocity_magnitude > max_value {
                let component_signs = SpatialVector([
                    rotation_change_velocity[0].signum(),
                    rotation_change_velocity[1].signum(),
                    rotation_change_velocity[2].signum(),
                ]);

                let rotation_change_velocity_normalized = rotation_change_velocity.normalize();

                let rotation_change_velocity_scaled = rotation_change_velocity_normalized * max_value;

                let rotation_change_velocity_scaled = SpatialVector([
                    rotation_change_velocity_scaled[0].abs() * component_signs[0],
                    rotation_change_velocity_scaled[1].abs() * component_signs[1],
                    rotation_change_velocity_scaled[2].abs() * component_signs[2],
                ]);

                self.previous_rotation + rotation_change_velocity_scaled * time_step
            } else {
                rotation_raw
            }
        } else {
            rotation_raw
        }

    }

    fn zero_velocity(&self) -> bool {
        self.translational_velocity_x == 0.0 &&
        self.translational_velocity_y == 0.0 &&
        self.translational_velocity_z == 0.0 &&
        self.wind_velocity == 0.0
    }

    fn translation(&self, time_step: f64) -> SpatialVector<3> {
        let x_position = self.upscaled_length_value(self.x_position);
        let y_position = self.upscaled_length_value(self.y_position);
        let z_position = self.upscaled_length_value(self.z_position);

        let current_position_raw = SpatialVector([x_position, y_position, z_position]);

        let position_change_velocity = (current_position_raw - self.previous_position) / time_step;

        let position_change_velocity_magnitude = position_change_velocity.length();

        if let Some(max_value) = self.parameters.max_position_change_velocity {
            if position_change_velocity_magnitude > max_value {
                let component_signs = SpatialVector([
                    position_change_velocity[0].signum(),
                    position_change_velocity[1].signum(),
                    position_change_velocity[2].signum(),
                ]);

                let position_change_velocity_normalized = position_change_velocity.normalize();

                let position_change_velocity_scaled = position_change_velocity_normalized * max_value;

                let position_change_velocity_scaled = SpatialVector([
                    position_change_velocity_scaled[0].abs() * component_signs[0],
                    position_change_velocity_scaled[1].abs() * component_signs[1],
                    position_change_velocity_scaled[2].abs() * component_signs[2],
                ]);

                self.previous_position + position_change_velocity_scaled * time_step
            } else {
                current_position_raw
            }
        } else {
            current_position_raw
        }

    }

    fn get_wind_direction(&self) -> f64 {
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

    fn translational_velocity(&self, time_step: f64) -> SpatialVector<3> {
        let translational_velocity_x = self.upscaled_velocity_value(self.translational_velocity_x);
        let translational_velocity_y = self.upscaled_velocity_value(self.translational_velocity_y);
        let translational_velocity_z = self.upscaled_velocity_value(self.translational_velocity_z);

        let mut translational_velocity = SpatialVector([
            translational_velocity_x,
            translational_velocity_y,
            translational_velocity_z,
        ]);
        
        if let Some(model) = &self.stormbird_model {
            if self.parameters.translational_velocity_in_body_fixed_frame {
                translational_velocity = translational_velocity.from_rotated_to_global_system(
                    self.rotation(time_step),
                    model.line_force_model.rotation_type
                );
            }
        }
        

        if self.parameters.reverse_translational_velocity {
            translational_velocity *= -1.0;
        }

        translational_velocity
    }

    fn velocity_input(&self, time_step: f64) -> Vec<SpatialVector<3>> {
        let wind_direction = self.get_wind_direction();

        let freestream_velocity_points: Vec<SpatialVector<3>> =
            if let Some(model) = &self.stormbird_model {
                model.get_freestream_velocity_points()
            } else {
                vec![]
            };

        let translational_velocity = self.translational_velocity(time_step);

        let mut velocity_input: Vec<SpatialVector<3>> = freestream_velocity_points.iter().map(
            |point| {
                let height = if self.parameters.negative_z_is_up {
                    -point[2]
                } else {
                    point[2]
                };

                let increase_factor = if let Some(model) = &self.height_variation_model {
                    model.velocity_increase_factor(height)
                } else {
                    1.0
                };

                let rotation_axis = SpatialVector([0.0, 0.0, 1.0]);

                let wind = SpatialVector([
                    -self.wind_velocity * increase_factor,
                    0.0,
                    0.0,
                ]).rotate_around_axis(wind_direction, rotation_axis);

                wind + translational_velocity
            }
        ).collect();

        if let Some(max_value) = self.parameters.max_input_velocity {
            for i in 0..velocity_input.len() {
                let velocity = velocity_input[i].length();

                let component_signs = SpatialVector([
                    velocity_input[i][0].signum(),
                    velocity_input[i][1].signum(),
                    velocity_input[i][2].signum(),
                ]);

                if velocity > max_value {
                    velocity_input[i] = velocity_input[i].normalize() * max_value;
                }

                velocity_input[i][0] = velocity_input[i][0].abs() * component_signs[0];
                velocity_input[i][1] = velocity_input[i][1].abs() * component_signs[1];
                velocity_input[i][2] = velocity_input[i][2].abs() * component_signs[2];
            }
        }

        velocity_input
    }

    fn local_wing_angles(&self) -> Vec<f64> {
        let mut local_wing_angles: Vec<f64> = vec![0.0; self.nr_wings];

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

        for i in 0..self.nr_wings {
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

        let mut section_models_internal_state = vec![0.0; self.nr_wings];

        for i in 0..self.nr_wings {
            section_models_internal_state[i] = section_models_internal_state_raw[i];
        }
        
        section_models_internal_state
    }

    /// Function that measures the angle of the velocity vector, with respect to the x-axis, at the
    /// specified non-dimensional spanwise location of each wing.
    ///
    /// That is, this angle can be seen as the felt wind direction at each sail.
    ///
    /// A typical case is that the Stormbird model is set up with a local body-fixed coordinate
    /// system. In that case, the measured wind-direction will also be in the local coordinate
    /// system
    fn measure_felt_wind_direction(&self, velocity: &[SpatialVector<3>]) -> Vec<f64> {
        let mut wind_directions = if let Some(model) = &self.stormbird_model {
            let relevant_velocities = model.line_force_model.interpolate_values_to_spanwise_location(
                self.parameters.non_dim_spanwise_measurement_position,
                velocity
            );

            let reference_vector = SpatialVector([-1.0, 0.0, 0.0]); // Negative x axis, as the angle is assumed to be 'coming from'

            // TODO: consider if it makes more sense to use an axis corrected for roll and pitch?
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
            vec![0.0; self.nr_wings]
        };

        if self.parameters.angles_in_degrees {
            for i in 0..wind_directions.len() {
                wind_directions[i] = wind_directions[i].to_degrees();
            }
        }

        wind_directions
    }

    pub fn measure_apparent_wind_direction(&self, velocity: &[SpatialVector<3>], time_step: f64) -> Vec<f64> {
        let mut apparent_wind_direction = if let Some(model) = &self.stormbird_model {
            let felt_velocity = model.line_force_model.felt_ctrl_points_freestream(velocity, time_step);

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
            vec![0.0; self.nr_wings]
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
        let mut angles_of_attack = if let Some(model) = &self.stormbird_model {
            model.line_force_model.interpolate_values_to_spanwise_location(
                self.parameters.non_dim_spanwise_measurement_position,
                angles_of_attack
            )
        } else {
            vec![0.0; self.nr_wings]
        };

        if self.parameters.angles_in_degrees {
            for i in 0..angles_of_attack.len() {
                angles_of_attack[i] = angles_of_attack[i].to_degrees();
            }
        }

        angles_of_attack
    }

    fn set_output(&mut self, result: &SimulationResult, velocity_input: &[SpatialVector<3>], time_step: f64) {
        if let Some(model) = &self.stormbird_model {
            let velocity_input_at_line_force_models = &velocity_input[0..model.line_force_model.nr_span_lines()];

            let apparent_wind_directions = self.measure_apparent_wind_direction(
                velocity_input_at_line_force_models,
                time_step
            );

            self.estimated_apparent_wind_direction = apparent_wind_directions.iter()
                .sum::<f64>() / apparent_wind_directions.len() as f64;
        } else {
            self.estimated_apparent_wind_direction = 0.0;
        }
        
        let integrated_forces = result.integrated_forces_sum();
        let integrated_moments = result.integrated_moments_sum();

        self.force_x = self.downscaled_force_value(integrated_forces[0]);
        self.force_y = self.downscaled_force_value(integrated_forces[1]);
        self.force_z = self.downscaled_force_value(integrated_forces[2]);

        self.moment_x = self.downscaled_moment_value(integrated_moments[0]);
        self.moment_y = self.downscaled_moment_value(integrated_moments[1]);
        self.moment_z = self.downscaled_moment_value(integrated_moments[2]);

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

    pub fn send_result_to_visualization_server(&self, result: &SimulationResult, time: f64) {
        if let Some(client) = &self.visualization_client {
            let encoded_result: Vec<u8> = bincode::serialize(result).unwrap();

            let response = client.post(self.parameters.visualization_server_address.clone() + "/update-result-data")
                .header("Content-Type", "application/octet-stream")
                .body(encoded_result)
                .send();

            match response {
                Ok(_) => {},
                Err(e) => {
                    let error_string = format!("Error sending result to visualization server: {}", e);

                    print_to_file(&error_string, "visualization_error.txt");
                }
            }

            let mut fmu_output_dict: HashMap<String, f64> = HashMap::new();
            
            fmu_output_dict.insert("time".to_string(), time);
            fmu_output_dict.insert("force_x".to_string(), self.force_x);
            fmu_output_dict.insert("force_y".to_string(), self.force_y);
            fmu_output_dict.insert("force_z".to_string(), self.force_z);
            fmu_output_dict.insert("moment_x".to_string(), self.moment_x);
            fmu_output_dict.insert("moment_y".to_string(), self.moment_y);
            fmu_output_dict.insert("moment_z".to_string(), self.moment_z);
            fmu_output_dict.insert("estimated_apparent_wind_direction".to_string(), self.estimated_apparent_wind_direction);
            fmu_output_dict.insert("angle_of_attack_measurement_1".to_string(), self.angle_of_attack_measurement_1);
            fmu_output_dict.insert("angle_of_attack_measurement_2".to_string(), self.angle_of_attack_measurement_2);
            fmu_output_dict.insert("angle_of_attack_measurement_3".to_string(), self.angle_of_attack_measurement_3);
            fmu_output_dict.insert("angle_of_attack_measurement_4".to_string(), self.angle_of_attack_measurement_4);
            fmu_output_dict.insert("angle_of_attack_measurement_5".to_string(), self.angle_of_attack_measurement_5);
            fmu_output_dict.insert("angle_of_attack_measurement_6".to_string(), self.angle_of_attack_measurement_6);
            fmu_output_dict.insert("angle_of_attack_measurement_7".to_string(), self.angle_of_attack_measurement_7);
            fmu_output_dict.insert("angle_of_attack_measurement_8".to_string(), self.angle_of_attack_measurement_8);
            fmu_output_dict.insert("angle_of_attack_measurement_9".to_string(), self.angle_of_attack_measurement_9);
            fmu_output_dict.insert("angle_of_attack_measurement_10".to_string(), self.angle_of_attack_measurement_10);

            let encoded_fmu_output_dict: Vec<u8> = bincode::serialize(&fmu_output_dict).unwrap();

            let response = client.post(self.parameters.visualization_server_address.clone() + "/update-fmu-output-data")
                .header("Content-Type", "application/octet-stream")
                .body(encoded_fmu_output_dict)
                .send();

            match response {
                Ok(_) => {},
                Err(e) => {
                    let error_string = format!("Error sending FMU output data to visualization server: {}", e);

                    print_to_file(&error_string, "visualization_error.txt");
                }
            }

            /*let plotly_wake = self.stormbird_model.as_ref().unwrap().wake.export_to_plotly_mesh();
            let encoded_plotly_wake: Vec<u8> = bincode::serialize(&plotly_wake).unwrap();

            let response = client.post(self.parameters.visualization_server_address.clone() + "/update-wake-shape")
                .header("Content-Type", "application/octet-stream")
                .body(encoded_plotly_wake)
                .send();

            match response {
                Ok(_) => {},
                Err(e) => {
                    let error_string = format!("Error sending wake shape to visualization server: {}", e);

                    print_to_file(&error_string, "visualization_error.txt");
                }
            }*/
        }
    }
}
