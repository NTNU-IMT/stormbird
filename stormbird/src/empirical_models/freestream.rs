// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to represent the freestream velocity in a simulation

use stormath::spatial_vector::SpatialVector;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Structure to model an atmospheric boundary layer according to a power law
pub struct PowerModelABL {
    /// A constant velocity component, independent of position. Primarily meant to represent the 
    /// velocity due to the forward motion of a vessel.
    pub constant_velocity: SpatialVector<3>,
    /// The reference wind velocity at the reference height. This value is used to as input when 
    /// computing how the wind velocity varies with height.
    pub reference_wind_velocity: SpatialVector<3>,
    #[serde(default="PowerModelABL::default_reference_height")]
    /// Reference height for the input reference wind velocity.
    pub reference_height: f64,
    #[serde(default="PowerModelABL::default_power_factor")]
    /// Power factor for the power law. 
    pub power_factor: f64,
    #[serde(default="PowerModelABL::default_up_direction")]
    /// The up direction in the simulation. This is used to compute the height of a location.
    pub up_direction: SpatialVector<3>,
    #[serde(default)]
    /// Reference value for the water plane height, used in cases where the origin of the coordinate
    /// system does not match the water plane location.
    pub water_plane_height: f64,
}

impl PowerModelABL {
    fn default_reference_height() -> f64 {10.0}
    fn default_power_factor() -> f64 {1.0/9.0}
    fn default_up_direction() -> SpatialVector<3> {SpatialVector::<3>::new(0.0, 0.0, 1.0)}

    pub fn velocity_at_location(&self, location: &SpatialVector<3>) -> SpatialVector<3> {
        let height = location.dot(self.up_direction) - self.water_plane_height;

        if height < 0.0 {
            dbg!(location, self.up_direction);
            panic!("The calculated height in the freestream input is negative. This is likely due to the up direction not being correctly set.")
        }

        let increase_factor = if self.power_factor > 0.0 {
            (height / self.reference_height).powf(self.power_factor)
        } else {
            1.0
        };
        
        self.constant_velocity + self.reference_wind_velocity * increase_factor
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Enum to store the freestream velocity using different approaches
pub enum Freestream {
    /// Constant freestream velocity
    Constant(SpatialVector<3>),
    /// Freestream velocity that varies with time
    PowerModelABL(PowerModelABL),
}

impl Freestream {
    pub fn velocity_at_location(&self, location: &SpatialVector<3>) -> SpatialVector<3> {
        match self {
            Freestream::Constant(v) => v.clone(),
            Freestream::PowerModelABL(model) => model.velocity_at_location(location),
        }
    }

    pub fn velocity_at_locations(&self, locations: &[SpatialVector<3>]) -> Vec<SpatialVector<3>> {
        locations.iter().map(|loc| self.velocity_at_location(loc)).collect()
    }
}