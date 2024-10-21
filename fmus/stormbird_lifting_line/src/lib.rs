// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use std::f64::consts::PI;

use fmu_from_struct::prelude::*;

use math_utils::spatial_vector::SpatialVector;
use stormbird::lifting_line::simulation::Simulation;
use stormbird::lifting_line::simulation_builder::SimulationBuilder;
use stormbird::io_structs::result::SimulationResult;
use stormbird::empirical_models::wind_environment::height_variation::HeightVariationModel;

use serde_json;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
/// FMU for a lifting line model using the Stormbird library.
pub struct StormbirdLiftingLine {
    #[parameter]
    pub lifting_line_setup_file_path: String,
    pub wind_environment_setup_file_path: String,
    pub angles_in_degrees: bool,
    pub negative_z_is_up: bool,
    pub reverse_wind_direction: bool,
    pub export_stormbird_result: bool,
    #[input]
    pub wind_velocity: f64,
    pub wind_direction_coming_from: f64,
    pub x_position: f64,
    pub y_position: f64,
    pub z_position: f64,
    pub x_rotation: f64,
    pub y_rotation: f64,
    pub z_rotation: f64,
    pub local_wing_angles: String,
    pub section_models_internal_state: String,
    #[output]
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,
    pub stormbird_result: String,

    stormbird_model: Option<Simulation>,
    height_variation_model: Option<HeightVariationModel>,
    nr_wings: usize,
    nr_freestream_velocity_points: usize,
    initialized_wake_points: bool,
}

impl FmuFunctions for StormbirdLiftingLine {        
    fn exit_initialization_mode(&mut self) {
        let initial_wake_builder_velocity = SpatialVector([1e-6, 0.0, 0.0]);

        let stormbird_model_builder = SimulationBuilder::new_from_file(
            &self.lifting_line_setup_file_path
        ).unwrap();

        self.stormbird_model = Some(stormbird_model_builder.build(1.0, initial_wake_builder_velocity));

        if let Some(model) = &mut self.stormbird_model {
            let freestream_velocity_points = model.get_freestream_velocity_points();

            self.nr_freestream_velocity_points = freestream_velocity_points.len();
            self.nr_wings = model.line_force_model.nr_wings();
        }

        if !self.wind_environment_setup_file_path.is_empty() {
            let height_variation_model = HeightVariationModel::from_json_file(
                &self.wind_environment_setup_file_path
            );

            self.height_variation_model = Some(height_variation_model);
        }
    }
    fn do_step(&mut self, current_time: f64, time_step: f64) {
        let rotation = self.rotation();
        let translation = self.translation();
        let wind_velocities = self.wind_velocities();
        let local_wing_angles = self.local_wing_angles();
        let section_models_internal_state = self.section_models_internal_state();

        let result = if let Some(model) = &mut self.stormbird_model {        
            model.line_force_model.rotation = rotation;
            model.line_force_model.translation = translation;
            model.line_force_model.local_wing_angles = local_wing_angles;

            model.line_force_model.set_section_models_internal_state(
                &section_models_internal_state
            );
            
            if !self.initialized_wake_points {
                let average_wind_velocities = wind_velocities.iter().sum::<SpatialVector<3>>() / wind_velocities.len() as f64;

                model.wake.initialize(
                    &model.line_force_model,
                    average_wind_velocities,
                    time_step
                );

                self.initialized_wake_points = true;
            }

            let result = model.do_step(
                current_time, 
                time_step, 
                &wind_velocities
            );

            Some(result)
        } else {
            None
        };

        if let Some(result) = result {
            self.set_output(result);
        }
    }
}

impl StormbirdLiftingLine {
    fn rotation(&self) -> SpatialVector<3> {
        if self.angles_in_degrees {
            SpatialVector([
                self.x_rotation.to_radians(), 
                self.y_rotation.to_radians(), 
                self.z_rotation.to_radians()
            ])
        } else {
            SpatialVector([
                self.x_rotation, 
                self.y_rotation, 
                self.z_rotation
            ])
        }
    }

    fn translation(&self) -> SpatialVector<3> {
        SpatialVector([self.x_position, self.y_position, self.z_position])
    }

    fn wind_velocities(&self) -> Vec<SpatialVector<3>> {
        let mut wind_direction = if self.angles_in_degrees {
            self.wind_direction_coming_from.to_radians()
        } else {
            self.wind_direction_coming_from
        };

        if self.reverse_wind_direction {
            wind_direction *= -1.0;
        }

        // Ensure wind direction is within +/- 180 degrees
        if wind_direction < -PI {
            wind_direction += 2.0 * PI;
        } else if wind_direction > PI {
            wind_direction -= 2.0 * PI;
        }

        let freestream_velocity_points: Vec<SpatialVector<3>> = if let Some(model) = &self.stormbird_model {
            model.get_freestream_velocity_points()
        } else {
            vec![]
        };

        let mut wind_velocities: Vec<SpatialVector<3>> = Vec::with_capacity(freestream_velocity_points.len());

        for point in freestream_velocity_points {
            let height = if self.negative_z_is_up {
                -point[2]
            } else {
                point[2]
            };

            let increase_factor = if let Some(model) = &self.height_variation_model {
                model.velocity_increase_factor(height)
            } else {
                1.0
            };

            let local_wind_velocity = self.wind_velocity * increase_factor;

            wind_velocities.push(
                SpatialVector([
                    -local_wind_velocity * wind_direction.cos(), 
                    -local_wind_velocity * wind_direction.sin(), 
                    0.0
                ]) 
            );
        }

        wind_velocities
    }

    fn local_wing_angles(&self) -> Vec<f64> {
        let mut local_wing_angles: Vec<f64> = if !self.local_wing_angles.is_empty() {
            serde_json::from_str(&self.local_wing_angles).unwrap()
        } else {
            vec![0.0; self.nr_wings]
        };

        if self.angles_in_degrees {
            for i in 0..local_wing_angles.len() {
                local_wing_angles[i] = local_wing_angles[i].to_radians();
            }
        }

        local_wing_angles
    }

    pub fn section_models_internal_state(&self) -> Vec<f64> {
        if !self.section_models_internal_state.is_empty() {
            serde_json::from_str(&self.section_models_internal_state).unwrap()
        } else {
            vec![0.0; self.nr_wings]
        }
    }

    fn set_output(&mut self, result: SimulationResult) {
        let integrated_forces  = result.integrated_forces_sum();
        let integrated_moments = result.integrated_moments_sum();

        self.force_x = integrated_forces[0];
        self.force_y = integrated_forces[1];
        self.force_z = integrated_forces[2];

        self.moment_x = integrated_moments[0];
        self.moment_y = integrated_moments[1];
        self.moment_z = integrated_moments[2];

        if self.export_stormbird_result {
            self.stormbird_result = serde_json::to_string(&result).unwrap();
        }
        
    }
}