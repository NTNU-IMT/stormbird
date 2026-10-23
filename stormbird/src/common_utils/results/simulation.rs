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

use crate::common_utils::results::simplfied::SingleSailResult;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
/// Structures used to return results from simulations. 
pub struct SimulationResult {
    /// The time of the result
    pub time: Float,
    /// The control points of the line force model
    pub ctrl_points: Vec<SpatialVector>,
    /// The velocity at the control points used as input to the solver
    pub solver_input_ctrl_points_velocity: Vec<SpatialVector>,
    /// The input used to compute the sectional forces
    pub force_input: SectionalForcesInput,
    /// The forces on each line element
    pub sectional_forces: SectionalForces,
    /// The forces integrated over each wing
    pub integrated_forces: Vec<IntegratedValues>,
    /// The moments integrated over each wing
    pub integrated_moments: Vec<IntegratedValues>,
    /// The input power to each wing
    pub input_power: Vec<Float>,
    /// The number of iterations used by the solver
    pub iterations: usize,
    /// The residual of the final solution
    pub residual: Float,
    /// The indices of the line elements belonging to each wing
    pub wing_indices: Vec<Range<usize>>,
    /// The rigid body motion of the line force model
    pub rigid_body_motion: RigidBodyMotion
}

impl SimulationResult {
    /// Reads a history of results, stored as a JSON array, from a file
    pub fn result_history_from_file(file_path: &str) -> Result<Vec<SimulationResult>, Error> {
        let file = std::fs::File::open(file_path)?;

        let reader = std::io::BufReader::new(file);

        let serde_res = serde_json::from_reader(reader)?;

        Ok(serde_res)
    }
    
    /// Reads a single result, stored as JSON, from a file
    pub fn from_json(file_path: &str) -> Result<SimulationResult, Error> {
        let string = std::fs::read_to_string(file_path)?;
        
        let serde_res = serde_json::from_str(&string)?;
        
        Ok(serde_res)
    }

    /// Returns the sum of the total integrated forces on all wings
    pub fn integrated_forces_sum(&self) -> SpatialVector {
        let mut sum = SpatialVector::default();

        for i in 0..self.integrated_forces.len() {
            sum += self.integrated_forces[i].total;
        }
        
        sum
    }

    /// Returns the sum of the total integrated moments on all wings
    pub fn integrated_moments_sum(&self) -> SpatialVector {
        let mut sum = SpatialVector::default();

        for i in 0..self.integrated_moments.len() {
            sum += self.integrated_moments[i].total;
        }
        
        sum
    }
    
    /// Returns the sum of the input power to all wings
    pub fn input_power_sum(&self) -> Float {
        let mut sum = 0.0;
        
        for i in 0..self.input_power.len() {
            sum += self.input_power[i];
        }
        
        sum
    }

    /// Writes the result to a file as JSON
    pub fn write_to_file(&self, file_path: &str) -> std::io::Result<()> {
        let file = std::fs::File::create(file_path)?;
        let writer = std::io::BufWriter::new(file);

        serde_json::to_writer(writer, self)?;

        Ok(())
    }

    /// Returns the number of line elements
    pub fn nr_span_lines(&self) -> usize {
        self.ctrl_points.len()
    }

    /// Returns the number of wings
    pub fn nr_of_wings(&self) -> usize {
        self.integrated_forces.len()
    }

    /// Returns the angles of attack on the line elements of the given wing
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
    
    /// Returns the simplified results, with the total force, moment and input power for each wing
    pub fn as_simplified(&self) -> Vec<SingleSailResult> {
        let nr_wings = self.nr_of_wings();
        
        let mut out: Vec<SingleSailResult> = Vec::with_capacity(nr_wings);
        
        for i in 0..nr_wings {
            out.push(
                SingleSailResult {
                    force: self.integrated_forces[i].total,
                    moment: self.integrated_moments[i].total,
                    input_power: self.input_power[i]
                }
            )
        }
        
        out
    }

    /// Returns a header and a data string for a CSV file, with the time and the total forces and
    /// moments on each wing
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
