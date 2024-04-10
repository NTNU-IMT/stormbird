// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub use fmu_from_struct::prelude::*;

#[derive(Debug, Default, Clone, Fmu)]
#[fmi_version = 2]
/// Converts the wind velocity input from a Vesim Vessel FMU to the coordinate system used in 
/// Stormbird. 
/// 
/// Assumed coordinate system for both Vesim and Stormbird is as follows:
/// +x is towards the bow
/// +y is towards starboard
/// +z is downwards
pub struct VesimToStormbird {
    #[input]
    /// The apparent wind velocity in the vessel coordinate system. The velocity should be a 
    /// combination of the vessel velocity and the wind velocity.
    pub apparent_wind_velocity: f64,
    /// The apparent wind direction in the vessel coordinate system, for the velocity specified
    /// above. A value of 0.0 means that the wind is coming from the front of the vessel. A value of 
    /// of 90 degrees means that the wind is coming from the right side of the vessel.
    /// 
    /// That means: 
    /// 0 degrees gives a negative value for the u component of the wind velocity (the wind is going 
    /// from the bow to the stern)
    /// 90 degrees gives a negative value for the v component of the wind velocity (the wind is 
    /// going from the starboard side to the port side)
    /// 180 degrees gives a positive value for the u component of the wind velocity
    /// 270 degrees gives a positive value for the v component of the wind velocity
    pub apparent_wind_direction: f64,
    #[output]
    pub effective_u: f64,
    pub effective_v: f64,
}

impl FmuFunctions for VesimToStormbird {
    fn do_step(&mut self, _current_time: f64, _time_step: f64) {
        self.effective_u = -self.apparent_wind_velocity * self.apparent_wind_direction.to_radians().cos();
        self.effective_v = -self.apparent_wind_velocity * self.apparent_wind_direction.to_radians().sin();
    }
}