// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use fmu_from_struct::prelude::*;

use stormbird::vec3::Vec3;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
/// Converts global wind data and ship motion data from Vesim to Stormbird format.
pub struct VesimToStormbird {
    #[input]
    /// The wind velocity in the global coordinate system, i.e., the true wind. 
    pub global_wind_velocity: f64,
    /// The global wind direction where the wind is **coming from**. 0 means coming from north, 
    /// +/-180 means south, +90 degrees means east and -90/+270 means west.
    pub global_wind_direction: f64,
    /// The ship course in the global coordinate system. 0 means the ship is traveling north, +/-180 
    /// means south, +90 means east and -90/+270 means west.
    pub ship_course: f64,
    /// The ship drift angle. A positive value means that the ship is moving right relative to the
    /// ship heading.
    pub drift_angle: f64,
    /// The surge velocity of the ship in the local coordinate system. A positive value means that
    /// the ship is moving forward.
    pub surge_velocity: f64,
    /// The sway velocity of the ship in the local coordinate system. A positive value means that
    /// the ship is moving to the right.
    pub sway_velocity: f64,
    #[output]
    /// The motion velocity, i.e., a spatial constant velocity, in the x direction in the Stormbird 
    /// coordinate system. 
    pub constant_velocity_x: f64,
    /// The motion velocity, i.e., a spatial constant velocity, in the y direction in the Stormbird
    /// coordinate system.
    pub constant_velocity_y: f64,
    /// The effective wind velocity in the x direction in the Stormbird coordinate system. The 
    /// reference velocity is input to a model for the atmospheric boundary layer model of how the 
    /// wind varies with height.
    pub reference_wind_velocity_x: f64,
    /// The effective wind velocity in the y direction in the Stormbird coordinate system. The 
    /// reference velocity is input to a model for the atmospheric boundary layer model of how the 
    /// wind varies with height.
    pub reference_wind_velocity_y: f64,
}

impl FmuFunctions for VesimToStormbird {
    fn do_step(&mut self, _current_time: f64, _time_step: f64) {
        let wind_global_vector = Vec3 {
            x: -self.global_wind_velocity * self.global_wind_direction.to_radians().cos(),
            y: -self.global_wind_velocity * self.global_wind_direction.to_radians().sin(),
            z: 0.0,
        };

        let ship_heading = (self.ship_course + self.drift_angle).to_radians();

        let ship_x_axis_in_global = Vec3 {
            x: ship_heading.cos(),
            y: ship_heading.sin(),
            z: 0.0,
        };

        let ship_y_axis_in_global = Vec3 {
            x: -ship_heading.sin(),
            y: ship_heading.cos(),
            z: 0.0,
        };

        self.reference_wind_velocity_x = wind_global_vector.dot(ship_x_axis_in_global);
        self.reference_wind_velocity_y = wind_global_vector.dot(ship_y_axis_in_global);

        self.constant_velocity_x = -self.surge_velocity;
        self.constant_velocity_y = -self.sway_velocity;
    }
}