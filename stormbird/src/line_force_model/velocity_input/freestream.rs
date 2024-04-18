// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

/// Functionality to represent the freestream velocity in a simulation

use crate::vec3::Vec3;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Structure to model an atmospheric boundary layer according to a power law
pub struct PowerModelABL {
    pub reference_velocity: Vec3,
    #[serde(default="PowerModelABL::default_reference_height")]
    pub reference_height: f64,
    #[serde(default="PowerModelABL::default_power_factor")]
    pub power_factor: f64,
    #[serde(default="PowerModelABL::default_up_direction")]
    pub up_direction: Vec3,
}

impl PowerModelABL {
    fn default_reference_height() -> f64 {10.0}
    fn default_power_factor() -> f64 {1.0/9.0}
    fn default_up_direction() -> Vec3 {Vec3::new(0.0, 0.0, 1.0)}

    pub fn velocity_at_location(&self, location: &Vec3) -> Vec3 {
        let height = location.dot(self.up_direction);

        let factor = (height / self.reference_height).powf(self.power_factor);
        
        self.reference_velocity * factor
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Enum to store the freestream velocity using different approaches
pub enum Freestream {
    /// Constant freestream velocity
    Constant(Vec3),
    /// Freestream velocity that varies with time
    PowerModelABL(PowerModelABL),
}

impl Freestream {
    pub fn velocity_at_location(&self, location: &Vec3) -> Vec3 {
        match self {
            Freestream::Constant(v) => v.clone(),
            Freestream::PowerModelABL(model) => model.velocity_at_location(location),
        }
    }

    pub fn velocity_at_locations(&self, locations: &[Vec3]) -> Vec<Vec3> {
        locations.iter().map(|loc| self.velocity_at_location(loc)).collect()
    }
}