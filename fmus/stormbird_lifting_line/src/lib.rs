// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use fmu_from_struct::prelude::*;

use math_utils::spatial_vector::SpatialVector;
use stormbird::lifting_line::simulation::Simulation;
use stormbird::lifting_line::simulation_builder::SimulationBuilder;
use stormbird::io_structs::result::SimulationResult;

use serde_json;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
/// FMU for a lifting line model using the Stormbird library.
pub struct StormbirdLiftingLine {
    #[parameter]
    pub setup_file_path: String,
    pub angles_in_degrees: bool,
    pub use_wind_environment: bool,
    pub use_local_wing_angles: bool,
    pub write_stormbird_results: bool,
    pub stormbird_results_path: String,
    #[input]
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub wind_environment_velocities: String,
    pub translation_x: f64,
    pub translation_y: f64,
    pub translation_z: f64,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
    pub local_wing_angles: String,
    #[output]
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,
    pub freestream_velocity_points: String,
    pub ctrl_points_velocities: String,

    stormbird_model: Option<Simulation>,
    nr_wings: usize,
    nr_freestream_velocity_points: usize,
    initialized_wake_points: bool,
}

impl FmuFunctions for StormbirdLiftingLine {        
    fn exit_initialization_mode(&mut self) {
        let initial_wake_builder_velocity = SpatialVector([1e-6, 0.0, 0.0]);

        let stormbird_model_builder = SimulationBuilder::new_from_file(
            &self.setup_file_path
        ).unwrap();

        self.stormbird_model = Some(stormbird_model_builder.build(1.0, initial_wake_builder_velocity));

        if let Some(model) = &mut self.stormbird_model {
            let freestream_velocity_points = model.get_freestream_velocity_points();

            self.nr_freestream_velocity_points = freestream_velocity_points.len();
            self.nr_wings = model.line_force_model.nr_wings();
        }
    }
    fn do_step(&mut self, current_time: f64, time_step: f64) {
        let rotation = self.rotation();
        let translation = self.translation();
        let freestream_velocity = self.freestream_velocity();
        let local_wing_angles = self.local_wing_angles();

        assert!(freestream_velocity.len() == self.nr_freestream_velocity_points);

        let average_freestream_velocity = freestream_velocity.iter().sum::<SpatialVector<3>>() / freestream_velocity.len() as f64;

        let result = if let Some(model) = &mut self.stormbird_model {        
            model.line_force_model.translation = translation;
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
    
            if self.use_wind_environment {
                let freestream_velocity_points = model.get_freestream_velocity_points();
    
                self.freestream_velocity_points = serde_json::to_string(&freestream_velocity_points).unwrap();
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
        }
    }

    fn translation(&self) -> SpatialVector<3> {
        SpatialVector([
            self.translation_x, 
            self.translation_y, 
            self.translation_z
        ])
    }

    fn freestream_velocity(&self) -> Vec<SpatialVector<3>> {
        if self.use_wind_environment {
            serde_json::from_str(&self.wind_environment_velocities).unwrap()
        } else {
            vec![
                SpatialVector([self.velocity_x, self.velocity_y, self.velocity_z]); 
                self.nr_freestream_velocity_points
            ]
        }
    }

    fn local_wing_angles(&self) -> Vec<f64> {
        let mut local_wing_angles: Vec<f64> = if self.use_local_wing_angles {
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

        self.ctrl_points_velocities = serde_json::to_string(&result.force_input.velocity).unwrap();
    }
}