// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality for representing wings as "line objects", without any assumptions about how
//! lift-induced velocities are estimated. In other words, this part is common for all methods
//! available in the library, and therefore the foundation of all simulations.

use std::{f64::consts::PI, ops::Range};

use stormath::{
    spatial_vector::SpatialVector,
    spatial_vector::transformations::RotationType,
    statistics::mean, 
    interpolation::linear_interpolation,
    rigid_body_motion::RigidBodyMotion
};

pub mod builder;
pub mod data_access;
pub mod force_calculations;
pub mod value_mapping;
pub mod span_line;

pub mod corrections;
pub mod prelude;

#[cfg(test)]
mod tests;

use crate::common_utils::prelude::*;
use crate::section_models::SectionModel;
use crate::controllers::output::ControllerOutput;

use corrections::{
    circulation::CirculationCorrection,
    angle_of_attack::AngleOfAttackCorrection,
};
use builder::single_wing::SingleWing;
use span_line::*;

#[derive(Clone, Debug)]
/// The struct holds variables for a model that calculate the forces on wings, under the assumption
/// that they can be represented as a set of line elements. The intended use is in lifting line and
/// actuator line simulations.
pub struct LineForceModel {
    /// Vector of line segments that defines the span geometry of the wings. Each have its own start
    /// and end point, to allow for uncoupled analysis
    pub span_lines_local: Vec<SpanLine>,
    /// Vectors representing both the chord length and the direction of the chord for each span line
    pub chord_vectors_local: Vec<SpatialVector>,
    /// Two dimensional models for lift and drag coefficients for each wing in the model
    pub section_models: Vec<SectionModel>,
    /// Indices used to sort different wings from each other.
    pub wing_indices: Vec<Range<usize>>,
    /// Rigid body motion of the line force model
    pub rigid_body_motion: RigidBodyMotion,
    /// Vector used to store local angles for each wing. This can be used to rotate the wing along
    /// the span axis during a dynamic simulation. The typical example is changing the angle of
    /// attack on a wing sail due to changing apparent wind conditions.
    pub local_wing_angles: Vec<f64>,
    /// A vector that contains booleans that indicate whether the circulation should be zero at the
    /// ends or not. The variables are used both when initializing the circulation before a
    /// simulation and in cases where smoothing is applied to the circulation.
    /// The vector is structured as follows:
    /// - The first index is the wing index
    /// - The second index is the end index, where 0 means that start of the wind and 1 means the ¨
    /// end
    /// - When the boolean is false, the circulation is set to zero at the end, and when it is true,
    ///  the circulation is assumed to be non-zero.
    pub non_zero_circulation_at_ends: Vec<[bool; 2]>,
    /// Density used in force calculations
    pub density: f64,
    /// Optional correction that can be applied to the estimated circulation strength.
    pub circulation_correction: CirculationCorrection,
    /// Optional correction for the angle of attack
    pub angle_of_attack_correction: AngleOfAttackCorrection,
    /// The coordinate system to generate the output in. Variants consists of Global and Body.
    pub output_coordinate_system: CoordinateSystem,
}

impl Default for LineForceModel {
    fn default() -> Self {
        Self::new(Self::default_density())
    }
}

impl LineForceModel {
    /// Default density for air at sea level in kg/m^3
    pub fn default_density() -> f64 {
        1.225
    }

    /// Creates a new empty line force model. Wings can be added using the 
    /// [LineForceModel::add_wing] function.
    pub fn new(density: f64) -> LineForceModel {
        Self {
            span_lines_local: Vec::new(),
            chord_vectors_local: Vec::new(),
            section_models: Vec::new(),
            wing_indices: Vec::new(),
            rigid_body_motion: RigidBodyMotion::default(),
            local_wing_angles: Vec::new(),
            non_zero_circulation_at_ends: Vec::new(),
            density,
            circulation_correction: Default::default(),
            angle_of_attack_correction: Default::default(),
            output_coordinate_system: CoordinateSystem::Global
        }
    }

    /// Adds a new wing to the complete model. This involves appending the span lines, chord vectors,
    /// and section models to the existing vectors, and adding the indices of the new wing to the
    /// wing indices vector.
    pub fn add_wing(&mut self, wing: &SingleWing) {
        let start_index = if self.span_lines_local.is_empty() {
            0
        } else {
            self.wing_indices.last().unwrap().end
        };

        let end_index = start_index + wing.span_lines_local.len();

        self.wing_indices.push(Range {
            start: start_index,
            end: end_index,
        });

        for line in &wing.span_lines_local {
            self.span_lines_local.push(line.clone());
        }

        for chord_vector in &wing.chord_vectors_local {
            self.chord_vectors_local.push(*chord_vector);
        }

        self.section_models.push(wing.section_model.clone());

        self.local_wing_angles.push(0.0);
        self.non_zero_circulation_at_ends
            .push(wing.non_zero_circulation_at_ends);
    }

    

    /// Finds the wing index from the global index of a line element.
    pub fn wing_index_from_global(&self, global_index: usize) -> usize {
        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            if wing_indices.contains(&global_index) {
                return wing_index;
            }
        }

        panic!("Wing index not found. The global index is not part of any wing")
    }

    /// Returns the local index of a single wing. For instance, the first line element of each wing
    /// will have a local index of 0, etc. This function is primarily used to map a global index to
    /// a local index used as input to sectional model where the properties vary for each line
    /// element.
    pub fn local_index_from_global(&self, global_index: usize) -> usize {
        for wing_indices in &self.wing_indices {
            if wing_indices.contains(&global_index) {
                return global_index - wing_indices.start;
            }
        }

        panic!("Local index not found. The global index is not part of any wing")
    }

    

    /// Resets the local wing angles to zero.
    pub fn reset_local_wing_angles(&mut self) {
        for angle in self.local_wing_angles.iter_mut() {
            *angle = 0.0;
        }
    }

    /// Removes the velocity in the span direction from the input velocity vector.
    pub fn remove_span_velocity(
        &self,
        velocity: &[SpatialVector],
        input_coordinate_system: CoordinateSystem,
    ) -> Vec<SpatialVector> {
        let span_lines = match input_coordinate_system {
            CoordinateSystem::Global => self.span_lines(),
            CoordinateSystem::Body => self.span_lines_local.clone(),
        };

        velocity
            .iter()
            .zip(span_lines.iter())
            .map(|(vel, line)| {
                let span_velocity = vel.project(line.relative_vector());

                *vel - span_velocity
            })
            .collect()
    }

    /// Calculates the wake angle behind each line element.
    pub fn wake_angles(&self, velocity: &[SpatialVector]) -> Vec<f64> {
        (0..self.nr_span_lines())
            .map(|index| {
                let wing_index = self.wing_index_from_global(index);

                match &self.section_models[wing_index] {
                    SectionModel::Foil(_) => 0.0,
                    SectionModel::VaryingFoil(_) => 0.0,
                    SectionModel::RotatingCylinder(cylinder) => cylinder.wake_angle(
                        self.chord_vectors_local[index].length(),
                        velocity[index].length(),
                    ),
                }
            })
            .collect()
    }    
    
    /// Shorthand for quickly calculating the typical force factor used when presenting
    /// non-dimensional forces from a simulation (i.e., lift and drag coefficients)
    pub fn total_force_factor(&self, freestream_velocity: f64) -> f64 {
        0.5 * self.density * freestream_velocity.powi(2) * self.total_projected_area()
    }

    pub fn set_section_models_internal_state(&mut self, internal_state: &[f64]) {
        for wing_index in 0..self.nr_wings() {
            match self.section_models[wing_index] {
                SectionModel::Foil(_) => {}
                SectionModel::VaryingFoil(ref mut foil) => {
                    foil.current_internal_state = internal_state[wing_index];
                }
                SectionModel::RotatingCylinder(ref mut cylinder) => {
                    cylinder.revolutions_per_second = internal_state[wing_index];
                }
            }
        }
    }

    pub fn set_controller_output(&mut self, controller_output: &ControllerOutput) {
        if let Some(local_wing_angles) = &controller_output.local_wing_angles {
            for (index, angle) in local_wing_angles.iter().enumerate() {
                self.local_wing_angles[index] = *angle;
            }
        }

        if let Some(section_models_internal_state) = &controller_output.section_models_internal_state {
            self.set_section_models_internal_state(section_models_internal_state);
        }
    }
}
