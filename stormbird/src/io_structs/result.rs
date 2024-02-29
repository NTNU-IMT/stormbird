// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Structures used to return results from simulations. Stored in their own module as they are 
//! used for both lifting line and actuator line simulations

use crate::vec3::Vec3;
use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub ctrl_points: Vec<Vec3>,
    pub circulation_strength: Vec<f64>,
    pub velocity: Vec<Vec3>,
    pub sectional_forces: Vec<Vec3>,
    pub integrated_forces: Vec<Vec3>,
    pub integrated_moments: Vec<Vec3>,
}

impl SimulationResult {
    pub fn new() -> Self {
        Self {
            ctrl_points: Vec::new(),
            circulation_strength: Vec::new(),
            velocity: Vec::new(),
            sectional_forces: Vec::new(),
            integrated_forces: Vec::new(),
            integrated_moments: Vec::new(),
        }
    }
    
    pub fn integrated_forces_sum(&self) -> Vec3 {
        self.integrated_forces.iter().sum()
    }

    pub fn integrated_moments_sum(&self) -> Vec3 {
        self.integrated_moments.iter().sum()
    }
}