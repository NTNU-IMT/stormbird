// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use std::ops::Range;

use stormath::{
    type_aliases::Float,
    spatial_vector::SpatialVector,
    rigid_body_motion::RigidBodyMotion,
};
use serde::{Serialize, Deserialize};

use crate::error::Error;

use crate::common_utils::forces_and_moments::{
    IntegratedValues,
    SectionalForces,
    SectionalForcesInput
};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Structures used to return results from simulations. 
pub struct SimulationResult {
    pub time: Float,
    pub ctrl_points: Vec<SpatialVector>,
    pub solver_input_ctrl_points_velocity: Vec<SpatialVector>,
    pub force_input: SectionalForcesInput,
    pub sectional_forces: SectionalForces,
    pub integrated_forces: Vec<IntegratedValues>,
    pub integrated_moments: Vec<IntegratedValues>,
    pub input_power: Vec<Float>,
    pub iterations: usize,
    pub residual: Float,
    pub wing_indices: Vec<Range<usize>>,
    pub rigid_body_motion: RigidBodyMotion
}

impl SimulationResult {
    pub fn result_history_from_file(file_path: &str) -> Result<Vec<SimulationResult>, Error> {
        let file = std::fs::File::open(file_path)?;

        let reader = std::io::BufReader::new(file);

        let serde_res = serde_json::from_reader(reader)?;

        Ok(serde_res)
    }

    pub fn integrated_forces_sum(&self) -> SpatialVector {
        let mut sum = SpatialVector::default();

        for i in 0..self.integrated_forces.len() {
            sum += self.integrated_forces[i].total;
        }
        
        sum
    }

    pub fn integrated_moments_sum(&self) -> SpatialVector {
        let mut sum = SpatialVector::default();

        for i in 0..self.integrated_moments.len() {
            sum += self.integrated_moments[i].total;
        }
        
        sum
    }
    
    pub fn input_power_sum(&self) -> Float {
        let mut sum = 0.0;
        
        for i in 0..self.input_power.len() {
            sum += self.input_power[i];
        }
        
        sum
    }

    pub fn write_to_file(&self, file_path: &str) -> std::io::Result<()> {
        let file = std::fs::File::create(file_path)?;
        let writer = std::io::BufWriter::new(file);

        serde_json::to_writer(writer, self)?;

        Ok(())
    }

    pub fn nr_span_lines(&self) -> usize {
        self.ctrl_points.len()
    }

    pub fn nr_of_wings(&self) -> usize {
        self.integrated_forces.len()
    }

    pub fn angles_of_attack_for_wing(&self, wing_index: usize) -> Vec<Float> {
        let mut angles_of_attack = Vec::new();

        for i in self.wing_indices[wing_index].start..self.wing_indices[wing_index].end {
            angles_of_attack.push(self.force_input.angles_of_attack[i]);
        }

        angles_of_attack
    }

    /// Returns the felt velocity at each control point, but with the motion due to rotational 
    /// motion subtracted. 
    pub fn felt_velocity_minus_rotational_motion(&self) -> Vec<SpatialVector> {
        let nr_span_lines = self.nr_span_lines();
        let mut out: Vec<SpatialVector> = Vec::with_capacity(nr_span_lines);

        for i in 0..nr_span_lines {
            out.push(
                self.force_input.velocity[i] + 
                self.rigid_body_motion.rotation_velocity_at_point(self.ctrl_points[i])
            )
        }

        out
    }

    /// Returns the felt input velocity at each control point, but with the motion due to rotational 
    /// motion subtracted. 
    pub fn felt_input_velocity_minus_rotational_motion(&self) -> Vec<SpatialVector> {
        let nr_span_lines = self.nr_span_lines();
        let mut out: Vec<SpatialVector> = Vec::with_capacity(nr_span_lines);

        for i in 0..nr_span_lines {
            out.push(
                self.solver_input_ctrl_points_velocity[i] + 
                self.rigid_body_motion.rotation_velocity_at_point(self.ctrl_points[i])
            )
        }

        out
    }

    pub fn as_reduced_flatten_csv_string(&self) -> (String, String) {
        let mut header = String::new();
        let mut data = String::new();

        header.push_str("time,");
        data.push_str(&format!("{}, ", self.time));

        for wing_index in 0..self.nr_of_wings() {
            header.push_str(&format!("force_{}.x,", wing_index));
            header.push_str(&format!("force_{}.y,", wing_index));
            header.push_str(&format!("force_{}.z,", wing_index));

            header.push_str(&format!("moment_{}.x,", wing_index));
            header.push_str(&format!("moment_{}.y,", wing_index));

            if wing_index == self.nr_of_wings() - 1 {
                header.push_str(&format!("moment_{}.z", wing_index));
            } else {
                header.push_str(&format!("moment_{}.z,", wing_index));
            }

            data.push_str(&format!("{},", self.integrated_forces[wing_index].total[0]));
            data.push_str(&format!("{},", self.integrated_forces[wing_index].total[1]));
            data.push_str(&format!("{},", self.integrated_forces[wing_index].total[2]));

            data.push_str(&format!("{},", self.integrated_moments[wing_index].total[0]));
            data.push_str(&format!("{},", self.integrated_moments[wing_index].total[1]));

            if wing_index == self.nr_of_wings() - 1 {
                data.push_str(&format!("{}", self.integrated_moments[wing_index].total[2]));
            } else {
                data.push_str(&format!("{},", self.integrated_moments[wing_index].total[2]));
            }
        }

        (header, data)
    }
}
