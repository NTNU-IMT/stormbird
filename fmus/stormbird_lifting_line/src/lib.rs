// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use fmu_from_struct::prelude::*;

use math_utils::spatial_vector::SpatialVector;
use stormbird::lifting_line::simulation::Simulation;
use stormbird::lifting_line::simulation_builder::SimulationBuilder;

use std::f64::consts::PI;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct StormbirdLiftingLine {
    #[parameter]
    pub setup_file_path: String,
    pub angles_in_degrees: bool,
    pub use_relative_angle: bool,
    pub max_wing_rotation_velocity: f64,
    pub write_stormbird_results: bool,
    pub stormbird_results_path: String,
    #[input]
    pub constant_velocity_x: f64,
    pub constant_velocity_y: f64,
    pub constant_velocity_z: f64,
    pub reference_wind_velocity_x: f64,
    pub reference_wind_velocity_y: f64,
    pub reference_wind_velocity_z: f64,
    pub translation_x: f64,
    pub translation_y: f64,
    pub translation_z: f64,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
    pub relative_angle: f64,
    #[output]
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,

    up_direction: SpatialVector<3>,
    stormbird_model: Option<Simulation>,
    nr_freestream_velocity_points: usize,
}

impl FmuFunctions for StormbirdLiftingLine {
    fn do_step(&mut self, current_time: f64, time_step: f64) {
        let freestream_velocity = SpatialVector([
            self.reference_wind_velocity_x,
            self.reference_wind_velocity_y,
            self.reference_wind_velocity_z
        ]);

        if let None = self.stormbird_model {
            self.initialize_model(time_step, freestream_velocity);
        }

        let translation = SpatialVector([
            self.translation_x, 
            self.translation_y, 
            self.translation_z
        ]);

        let rotation = if self.angles_in_degrees {
            SpatialVector([
                self.rotation_x.to_radians(), 
                self.rotation_y.to_radians(), 
                self.rotation_z.to_radians()
            ])
        } else {
            SpatialVector([
                self.rotation_x, 
                self.rotation_y, 
                self.rotation_z
            ])
        };

        if self.use_relative_angle {
            self.apply_relative_angle(freestream_velocity, time_step);
        }
        
        if let Some(model) = &mut self.stormbird_model {
            model.line_force_model.translation = translation;
            model.line_force_model.rotation = rotation;

            let result = model.do_step(
                current_time, 
                time_step, 
                &vec![freestream_velocity; self.nr_freestream_velocity_points]
            );

            if self.write_stormbird_results {
                result.write_to_file(&self.stormbird_results_path).unwrap();
            }

            let integrated_forces  = result.integrated_forces_sum();
            let integrated_moments = result.integrated_moments_sum();

            self.force_x = integrated_forces[0];
            self.force_y = integrated_forces[1];
            self.force_z = integrated_forces[2];

            self.moment_x = integrated_moments[0];
            self.moment_y = integrated_moments[1];
            self.moment_z = integrated_moments[2];
        }
    }
}

impl StormbirdLiftingLine {
    fn initialize_model(&mut self, time_step: f64, freestream_velocity: SpatialVector<3>) {
        let stormbird_model_builder = SimulationBuilder::new_from_file(
            &self.setup_file_path
        ).unwrap();

        self.stormbird_model = Some(stormbird_model_builder.build(time_step, freestream_velocity));

        if let Some(model) = &mut self.stormbird_model {
            let span_lines = model.line_force_model.span_lines();
            
            self.up_direction = span_lines[0].relative_vector().normalize();

            let freestream_velocity_points = model.get_freestream_velocity_points();

            self.nr_freestream_velocity_points = freestream_velocity_points.len();
        }
    }
    /// Applies the relative angle to the model
    fn apply_relative_angle(&mut self, freestream_velocity: SpatialVector<3>, time_step: f64) {
        if let Some(model) = &mut self.stormbird_model{
            let relative_angle = if self.angles_in_degrees {
                self.relative_angle.to_radians()
            } else {
                self.relative_angle
            };

            let previous_angles = model.line_force_model.local_wing_angles.clone();

            let max_wing_rotation_velocity = if self.max_wing_rotation_velocity == 0.0 {
                359.0_f64.to_radians() / time_step
            } else {
                if self.angles_in_degrees {
                    self.max_wing_rotation_velocity.to_radians()
                } else {
                    self.max_wing_rotation_velocity
                }
            };
           
    
            model.line_force_model.reset_local_wing_angles();
        
            let average_chord_vectors = model.line_force_model.wing_averaged_values(
                &model.line_force_model.chord_vectors()
            );
    
            for wing_index in 0..model.line_force_model.nr_wings() {
                let rotation_axis = model.line_force_model.wing_rotation_axis(wing_index);
    
                let zero_angle = average_chord_vectors[wing_index].signed_angle_between(
                    freestream_velocity, rotation_axis
                );

                let new_estimated_angle = zero_angle + relative_angle;

                let delta_angle = (new_estimated_angle - previous_angles[wing_index]) % (2.0 * PI);

                let necessary_rotation_speed = delta_angle / time_step;

                let rotation_speed = necessary_rotation_speed
                    .min(max_wing_rotation_velocity)
                    .max(-max_wing_rotation_velocity);

                model.line_force_model.local_wing_angles[wing_index] = previous_angles[wing_index] + rotation_speed * time_step;
            }
        }       
    }
}