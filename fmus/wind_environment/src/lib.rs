// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use fmu_from_struct::prelude::*;

use math_utils::spatial_vector::SpatialVector;
use stormbird::empirical_models::freestream::PowerModelABL;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
pub struct WindEnvironment {
    #[parameter]
    pub power_law_coefficient: f64,
    pub reference_height: f64,
    pub up_direction_x: f64,
    pub up_direction_y: f64,
    pub up_direction_z: f64,
    pub use_list_1: bool,
    pub use_list_2: bool,
    #[input]
    pub points_list_1: String,
    pub points_list_2: String,
    #[output]
    pub velocity_list_1: String,
    pub velocity_list_2: String,

    power_model: Option<PowerModelABL>,
}

impl FmuFunctions for WindEnvironment {
    fn do_step(&mut self, current_time: f64, time_step: f64) {
        if self.power_model.is_none() {
            self.initialize_model();
        }

        if let Some(power_model) = &self.power_model {
            let points_list_1: Vec<SpatialVector> = serde_json::from_str(&self.points_list_1).unwrap();

            let velocity_list_1: Vec<SpatialVector> = points_list_1.iter().map(|point| 
                power_model.velocity_at_location(point)
            ).collect();

        }
    }
}

impl WindEnvironment {
    fn initialize_model(&mut self) {
        let up_direction = SpatialVector([self.up_direction_x, self.up_direction_y, self.up_direction_z]);
        
        let power_model = PowerModelABL {
            constant_velocity: SpatialVector([0.0, 0.0, 0.0]),
            reference_wind_velocity: SpatialVector([0.0, 0.0, 0.0]),
            reference_height: self.reference_height,
            power_factor: self.power_law_coefficient,
            up_direction,
            water_plane_height: 0.0,
        };

        self.power_model = Some(power_model);
    }
}
