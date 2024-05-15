// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Results from simulations.

use crate::vec3::Vec3;
use serde::{Serialize, Deserialize};

use super::forces_and_moments::{
    IntegratedValues,
    SectionalForces,
    SectionalForcesInput
};

#[derive(Debug, Clone)]
/// Results from a lifting line solver, which will be further used to generate SimulationResults
pub struct SolverResult {
    pub circulation_strength: Vec<f64>,
    pub ctrl_point_velocity: Vec<Vec3>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Structures used to return results from simulations. 
pub struct SimulationResult {
    pub ctrl_points: Vec<Vec3>,
    pub force_input: SectionalForcesInput,
    pub sectional_forces: SectionalForces,
    pub integrated_forces: Vec<IntegratedValues>,
    pub integrated_moments: Vec<IntegratedValues>,
}

impl SimulationResult {
    pub fn integrated_forces_sum(&self) -> Vec3 {
        let mut sum = Vec3::default();

        for i in 0..self.integrated_forces.len() {
            sum += self.integrated_forces[i].total;
        }
        
        sum
    }

    pub fn integrated_moments_sum(&self) -> Vec3 {
        let mut sum = Vec3::default();

        for i in 0..self.integrated_moments.len() {
            sum += self.integrated_moments[i].total;
        }
        
        sum
    }

    pub fn write_to_file(&self, file_path: &str) -> std::io::Result<()> {
        let file = std::fs::File::create(file_path)?;
        let writer = std::io::BufWriter::new(file);

        serde_json::to_writer(writer, self)?;

        Ok(())
    }
}