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
    pub water_plane_height: f64,
    pub up_direction_x: f64,
    pub up_direction_y: f64,
    pub up_direction_z: f64,
    pub use_list_1: bool,
    pub use_list_2: bool,
    #[input]
    pub constant_velocity_x: f64,
    pub constant_velocity_y: f64,
    pub constant_velocity_z: f64,
    pub reference_wind_velocity_x: f64,
    pub reference_wind_velocity_y: f64,
    pub reference_wind_velocity_z: f64,
    pub points_list_1: String,
    pub points_list_2: String,
    #[output]
    pub velocity_list_1: String,
    pub velocity_list_2: String,

    power_model: Option<PowerModelABL>,
}

impl FmuFunctions for WindEnvironment {
    fn do_step(&mut self, _current_time: f64, _time_step: f64) {
        if self.power_model.is_none() {
            self.initialize_model();
        }

        if let Some(power_model) = &self.power_model {
            if self.use_list_1 && !self.points_list_1.is_empty() {
                let points_list_1: Vec<SpatialVector<3>> = serde_json::from_str(&self.points_list_1).unwrap();

                let velocity_list_1: Vec<SpatialVector<3>> = points_list_1.iter().map(|point| 
                    power_model.velocity_at_location(point)
                ).collect();

                self.velocity_list_1 = serde_json::to_string(&velocity_list_1).unwrap();
            }

            if self.use_list_2 && !self.points_list_2.is_empty() {
                let points_list_2: Vec<SpatialVector<3>> = serde_json::from_str(&self.points_list_2).unwrap();

                let velocity_list_2: Vec<SpatialVector<3>> = points_list_2.iter().map(|point| 
                    power_model.velocity_at_location(point)
                ).collect();

                self.velocity_list_2 = serde_json::to_string(&velocity_list_2).unwrap();
            }
        }
    }
}

impl WindEnvironment {
    fn initialize_model(&mut self) {    
        let power_model = PowerModelABL {
            constant_velocity: self.constant_velocity(),
            reference_wind_velocity: self.reference_wind_velocity(),
            reference_height: self.reference_height,
            power_factor: self.power_law_coefficient,
            up_direction: self.up_direction(),
            water_plane_height: self.water_plane_height,
        };

        self.power_model = Some(power_model);
    }

    fn constant_velocity(&self) -> SpatialVector<3> {
        SpatialVector([self.constant_velocity_x, self.constant_velocity_y, self.constant_velocity_z])
    }

    fn reference_wind_velocity(&self) -> SpatialVector<3> {
        SpatialVector([self.reference_wind_velocity_x, self.reference_wind_velocity_y, self.reference_wind_velocity_z])
    }

    fn up_direction(&self) -> SpatialVector<3> {
        SpatialVector([self.up_direction_x, self.up_direction_y, self.up_direction_z])
    }
}
