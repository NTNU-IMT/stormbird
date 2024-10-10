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
    pub wind_angle_relative_measurement_height: f64,
    pub include_induced_velocities_in_wind_angle_measurements: bool,
    pub negative_z_is_up: bool,
    pub export_stormbird_result: bool,
    #[input]
    pub surge_velocity: f64,
    pub sway_velocity: f64,
    pub wind_velocity: f64,
    pub global_wind_direction: f64,
    pub global_ship_heading: f64,
    pub heave_position: f64,
    pub roll_angle: f64,
    pub pitch_angle: f64,
    pub local_wing_angles: String,
    #[output]
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,
    pub wind_angle_measurements: String, 
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
        let freestream_velocity = self.freestream_velocity();
        let local_wing_angles = self.local_wing_angles();

        let average_freestream_velocity = freestream_velocity.iter().sum::<SpatialVector<3>>() / freestream_velocity.len() as f64;

        let result = if let Some(model) = &mut self.stormbird_model {        
            model.line_force_model.rotation = rotation;
            model.line_force_model.local_wing_angles = local_wing_angles;
            
            if !self.initialized_wake_points {
                model.wake.initialize(
                    &model.line_force_model,
                    average_freestream_velocity,
                    time_step
                );

                self.initialized_wake_points = true;
            }

            let result = model.do_step(
                current_time, 
                time_step, 
                &freestream_velocity
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
                self.roll_angle.to_radians(), 
                self.pitch_angle.to_radians(), 
                0.0
            ])
        } else {
            SpatialVector([
                self.roll_angle, 
                self.pitch_angle, 
                0.0
            ])
        }
    }

    fn relative_wind_direction(&self) -> f64 {
        let mut dir = self.global_wind_direction - self.global_ship_heading;

        if self.angles_in_degrees {
            dir = dir.to_radians();
        }

        if dir < -PI {
            dir += 2.0 * PI;
        } else if dir > PI {
            dir -= 2.0 * PI;
        }

        dir
    }

    fn freestream_velocity(&self) -> Vec<SpatialVector<3>> {
        let wind_direction = self.relative_wind_direction();

        let freestream_velocity_points: Vec<SpatialVector<3>> = if let Some(model) = &self.stormbird_model {
            model.get_freestream_velocity_points()
        } else {
            vec![]
        };

        let mut freestream_velocity: Vec<SpatialVector<3>> = Vec::with_capacity(freestream_velocity_points.len());

        for point in freestream_velocity_points {
            let height = if self.negative_z_is_up {
                -point[2] - self.heave_position
            } else {
                point[2] + self.heave_position
            };

            let increase_factor = if let Some(model) = &self.height_variation_model {
                model.velocity_increase_factor(height)
            } else {
                1.0
            };

            let wind_velocity = self.wind_velocity * increase_factor;

            freestream_velocity.push(
                SpatialVector([
                    -self.surge_velocity + wind_velocity * wind_direction.cos(), 
                    -self.sway_velocity + wind_velocity * wind_direction.sin(), 
                    0.0
                ]) 
            );
        }

        freestream_velocity
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