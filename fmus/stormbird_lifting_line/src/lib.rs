// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub use fmu_from_struct::prelude::*;

use stormbird::lifting_line::prelude::*;
use stormbird::lifting_line::simulation::{Simulation, SimulationBuilder};
use stormbird::line_force_model::velocity_input::InputState;
use stormbird::line_force_model::velocity_input::freestream::Freestream;


#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct StormbirdLiftingLine {
    #[parameter]
    pub setup_file_path: String,
    pub angles_in_degrees: bool,
    pub use_relative_angle: bool,
    #[input]
    pub translation_x: f64,
    pub translation_y: f64,
    pub translation_z: f64,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
    pub freestream_u: f64,
    pub freestream_v: f64,
    pub freestream_w: f64,
    pub relative_angle: f64,
    #[output]
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub moment_x: f64,
    pub moment_y: f64,
    pub moment_z: f64,

    stormbird_model: Option<Simulation>,
}

impl FmuFunctions for StormbirdLiftingLine {
    fn exit_initialization_mode(&mut self) {
        let stormbird_model_builder = SimulationBuilder::new_from_file(&self.setup_file_path).unwrap();

        self.stormbird_model = Some(stormbird_model_builder.build());
    }

    fn do_step(&mut self, current_time: f64, time_step: f64) {
        let input_state = self.input_state();

        if self.use_relative_angle {
            let ctrl_points = self.stormbird_model.as_ref().unwrap().line_force_model.ctrl_points();

            let freestream_velocity = input_state.freestream.velocity_at_locations(
                &ctrl_points
            );

            let average_freestream_velocity = freestream_velocity.iter().sum::<Vec3>() / freestream_velocity.len() as f64;

            self.apply_relative_angle(average_freestream_velocity);
        }
        
        if let Some(model) = &mut self.stormbird_model {
            let result = model.do_step(current_time, time_step, input_state);

            let integrated_forces  = result.integrated_forces_sum();
            let integrated_moments = result.integrated_moments_sum();

            self.force_x = integrated_forces.x;
            self.force_y = integrated_forces.y;
            self.force_z = integrated_forces.z;

            self.moment_x = integrated_moments.x;
            self.moment_y = integrated_moments.y;
            self.moment_z = integrated_moments.z;
        }
    }
}

impl StormbirdLiftingLine {
    fn input_state(&self) -> InputState {
        let translation = Vec3::new(
            self.translation_x, 
            self.translation_y, 
            self.translation_z
        );

        let rotation = if self.angles_in_degrees {
            Vec3::new(
                self.rotation_x.to_radians(), 
                self.rotation_y.to_radians(), 
                self.rotation_z.to_radians()
            )
        } else {
            Vec3::new(
                self.rotation_x, 
                self.rotation_y, 
                self.rotation_z
            )
        };

        let freestream = Freestream::Constant(
            Vec3::new(
                self.freestream_u, 
                self.freestream_v, 
                self.freestream_w
            )
        );

        InputState {
            freestream,
            translation,
            rotation,
        }
    }


    /// Applies the relative angle to the model
    fn apply_relative_angle(&mut self, freestream_velocity: Vec3) {
        if let Some(model) = &mut self.stormbird_model{
            let relative_angle = if self.angles_in_degrees {
                self.relative_angle.to_radians()
            } else {
                self.relative_angle
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
    
                model.line_force_model.local_wing_angles[wing_index] = zero_angle + relative_angle;
            }
        }       
    }
}