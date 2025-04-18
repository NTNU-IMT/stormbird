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
    interpolation::linear_interpolation};

pub mod builder;
pub mod force_calculations;
pub mod span_line;

pub mod circulation_corrections;
pub mod prelude;
pub mod single_wing;

pub mod rigid_body_motion;

#[cfg(test)]
mod tests;

use crate::common_utils::prelude::*;
use crate::section_models::SectionModel;

use self::rigid_body_motion::RigidBodyMotion;

use circulation_corrections::CirculationCorrection;
use single_wing::SingleWing;
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
    pub chord_vectors_local: Vec<SpatialVector<3>>,
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
    /// Optional corrections that can be applied to the estimated circulation strength.
    pub circulation_corrections: CirculationCorrection,
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
            circulation_corrections: Default::default(),
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

    /// Short hand for querying for the number of wings in the model
    pub fn nr_wings(&self) -> usize {
        self.wing_indices.len()
    }

    /// Short hand for querying for the number of span lines in the model
    pub fn nr_span_lines(&self) -> usize {
        self.span_lines_local.len()
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

    /// Returns the axis of rotation for the wing at the input index.
    pub fn wing_rotation_axis(&self, wing_index: usize) -> SpatialVector<3> {
        self.span_lines_local[self.wing_indices[wing_index].start].relative_vector()
    }

    pub fn wing_rotation_axis_from_global(&self, global_index: usize) -> SpatialVector<3> {
        let wing_index = self.wing_index_from_global(global_index);

        self.wing_rotation_axis(wing_index)
    }

    /// Returns both angle and axis of rotation for the wing at the input index.
    pub fn wing_rotation_data(&self, wing_index: usize) -> (f64, SpatialVector<3>) {
        let axis = self.wing_rotation_axis(wing_index);
        let angle = self.local_wing_angles[wing_index];

        (angle, axis)
    }

    pub fn wing_rotation_data_from_global(&self, global_index: usize) -> (f64, SpatialVector<3>) {
        let wing_index = self.wing_index_from_global(global_index);

        self.wing_rotation_data(wing_index)
    }

    /// Resets the local wing angles to zero.
    pub fn reset_local_wing_angles(&mut self) {
        for angle in self.local_wing_angles.iter_mut() {
            *angle = 0.0;
        }
    }

    pub fn span_line_at_index(&self, index: usize) -> SpanLine {
        let mut span_line = self.span_lines_local[index].clone();

        span_line.start_point = self.rigid_body_motion.transform_point(span_line.start_point);
        span_line.end_point = self.rigid_body_motion.transform_point(span_line.end_point);

        span_line
    }

    /// Returns the span lines in global coordinates.
    pub fn span_lines(&self) -> Vec<SpanLine> {
        (0..self.nr_span_lines())
            .map(|i| self.span_line_at_index(i))
            .collect()
    }

    pub fn local_chord_vector_at_index(&self, index: usize) -> SpatialVector<3> {
        let (angle, axis) = self.wing_rotation_data_from_global(index);

        self.chord_vectors_local[index].rotate_around_axis(angle, axis)
    }

    pub fn global_chord_vector_at_index(&self, index: usize) -> SpatialVector<3> {
        self.rigid_body_motion.transform_vector(self.local_chord_vector_at_index(index))
    }

    /// Returns the chord vectors in global coordinates.
    pub fn local_chord_vectors(&self) -> Vec<SpatialVector<3>> {
        self.chord_vectors_local
            .iter()
            .enumerate()
            .map(|(global_index, chord_vector)| {
                let (angle, axis) = self.wing_rotation_data_from_global(global_index);

                chord_vector.rotate_around_axis(angle, axis)
            })
            .collect()
    }

    pub fn global_chord_vectors(&self) -> Vec<SpatialVector<3>> {
        let local_chord_vectors = self.local_chord_vectors();

        local_chord_vectors
            .iter()
            .map(|chord_vector| self.rigid_body_motion.transform_vector(*chord_vector))
            .collect()
    }

    /// Returns the control points of each line element. This is calculated as the midpoint of each
    /// span line
    pub fn ctrl_points(&self) -> Vec<SpatialVector<3>> {
        let span_lines = self.span_lines();

        span_lines
            .iter()
            .map(|line| line.ctrl_point())
            .collect()
    }

    /// Returns the control points of each line element in local coordinates. This is calculated as
    /// the midpoint of each span line
    pub fn ctrl_points_local(&self) -> Vec<SpatialVector<3>> {
        self.span_lines_local
            .iter()
            .map(|line| line.ctrl_point())
            .collect()
    }

    /// Returns the points making up the line geometry of the wings as a vector of spatial vectors,
    /// as opposed to a vector of span lines.
    pub fn span_points(&self) -> Vec<SpatialVector<3>> {
        let span_lines = self.span_lines();
        let mut span_points: Vec<SpatialVector<3>> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {
            for i in self.wing_indices[wing_index].clone() {
                span_points.push(span_lines[i].start_point);
            }

            let last_index = self.wing_indices[wing_index].clone().last().unwrap();

            span_points.push(span_lines[last_index].end_point);
        }

        span_points
    }

    /// Removes the velocity in the span direction from the input velocity vector.
    pub fn remove_span_velocity(
        &self,
        velocity: &[SpatialVector<3>],
        input_coordinate_system: CoordinateSystem,
    ) -> Vec<SpatialVector<3>> {
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
    pub fn wake_angles(&self, velocity: &[SpatialVector<3>]) -> Vec<f64> {
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

    pub fn span_distance_in_local_coordinates(&self) -> Vec<f64> {
        let mut span_distance: Vec<f64> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {
            let start_point =
                self.span_lines_local[self.wing_indices[wing_index].start].start_point;

            let mut previous_point = start_point;
            let mut previous_distance = 0.0;

            let mut current_wing_span_distance: Vec<f64> = Vec::new();

            for i in self.wing_indices[wing_index].clone() {
                let line = &self.span_lines_local[i];

                let increase_in_distance = line.ctrl_point().distance(previous_point);
                previous_point = line.ctrl_point();

                current_wing_span_distance.push(previous_distance + increase_in_distance);

                previous_distance += increase_in_distance;
            }

            let end_point = self.span_lines_local
                [self.wing_indices[wing_index].clone().last().unwrap()]
            .end_point;

            let total_distance =
                current_wing_span_distance.last().unwrap() + end_point.distance(previous_point);

            for i in 0..self.wing_indices[wing_index].end - self.wing_indices[wing_index].start {
                span_distance.push(current_wing_span_distance[i] - 0.5 * total_distance);
            }
        }

        span_distance
    }

    /// Calculates the relative distance from the center off each wing for each control point.
    /// The absolute values are divided with the span of each wing. In other words, the
    /// return value will vary between -0.5 and 0.5, where 0 is the center of the wing.
    pub fn relative_span_distance(&self) -> Vec<f64> {
        let mut relative_span_distance: Vec<f64> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {
            let start_point =
                self.span_lines_local[self.wing_indices[wing_index].start].start_point;

            let mut previous_point = start_point;
            let mut previous_distance = 0.0;

            let mut current_wing_span_distance: Vec<f64> = Vec::new();

            for i in self.wing_indices[wing_index].clone() {
                let line = &self.span_lines_local[i];

                let increase_in_distance = line.ctrl_point().distance(previous_point);
                previous_point = line.ctrl_point();

                current_wing_span_distance.push(previous_distance + increase_in_distance);

                previous_distance += increase_in_distance;
            }

            let end_point = self.span_lines_local
                [self.wing_indices[wing_index].clone().last().unwrap()]
            .end_point;

            let total_distance =
                current_wing_span_distance.last().unwrap() + end_point.distance(previous_point);

            for i in 0..self.wing_indices[wing_index].end - self.wing_indices[wing_index].start {
                relative_span_distance.push(current_wing_span_distance[i] / total_distance - 0.5);
            }
        }

        relative_span_distance
    }

    /// Integrates the chord length along the span of all wings in the model to return the total
    /// projected area of the wing.
    pub fn total_projected_area(&self) -> f64 {
        let mut total_area = 0.0;

        for i in 0..self.nr_span_lines() {
            total_area += self.chord_vectors_local[i].length() * self.span_lines_local[i].length();
        }

        total_area
    }

    /// returns the span length of each wing in the model
    pub fn wing_span_lengths(&self) -> Vec<f64> {
        let mut span_length = vec![0.0; self.nr_wings()];

        for i in 0..self.nr_span_lines() {
            let wing_index = self.wing_index_from_global(i);

            span_length[wing_index] += self.span_lines_local[i].length();
        }

        span_length
    }

    /// Shorthand for quickly calculating the typical force factor used when presenting
    /// non-dimensional forces from a simulation (i.e., lift and drag coefficients)
    pub fn total_force_factor(&self, freestream_velocity: f64) -> f64 {
        0.5 * self.density * freestream_velocity.powi(2) * self.total_projected_area()
    }

    pub fn set_section_models_internal_state(&mut self, internal_state: &[f64]) {
        for wing_index in 0..self.nr_wings() {
            match self.section_models[wing_index] {
                SectionModel::VaryingFoil(ref mut foil) => {
                    foil.current_internal_state = internal_state[wing_index];
                }
                SectionModel::RotatingCylinder(ref mut cylinder) => {
                    cylinder.revolutions_per_second = internal_state[wing_index];
                }
                _ => {}
            }
        }
    }

    /// General function for calculating wing-averaged values
    pub fn wing_averaged_values<T>(&self, sectional_values: &[T]) -> Vec<T>
    where
        T: std::ops::Div<f64, Output = T> + std::ops::Add<T, Output = T> + Copy,
    {
        let mut result: Vec<T> = Vec::new();

        for wing_indices in &self.wing_indices {
            result.push(mean(&sectional_values[wing_indices.clone()]));
        }

        result
    }

    /// Function for interpolating vector values to a specified relative spanwise distance for each
    /// wing.
    pub fn interpolate_values_to_spanwise_location<T>(&self, spanwise_location: f64, sectional_values: &[T]) -> Vec<T>
    where T:
        std::ops::Mul<f64, Output = T> +
        std::ops::Add<T, Output = T> +
        std::ops::Sub<T, Output = T> +
        Copy
    {
        let mut result: Vec<T> = Vec::with_capacity(self.nr_wings());

        let relative_span_distance = self.relative_span_distance();

        for wing_indices in &self.wing_indices {
            result.push(
                linear_interpolation(
                    spanwise_location,
                    &relative_span_distance[wing_indices.clone()],
                    &sectional_values[wing_indices.clone()]
                )
            )
        }

        result
    }

    /// Maps a vector of values for each wing to a vector of values for each section
    pub fn section_values_from_wing_values<T>(&self, wing_values: &[T]) -> Vec<T>
    where
        T: Clone,
    {
        let mut result: Vec<T> = Vec::new();

        for (wing_nr, wing_indices) in self.wing_indices.iter().enumerate() {
            let wing_value = &wing_values[wing_nr];

            for _ in wing_indices.clone() {
                result.push(wing_value.clone());
            }
        }

        result
    }

    /// Maps the values at the control points to the values at the span points using linear
    /// interpolation.
    pub fn span_point_values_from_ctrl_point_values<T>(
        &self,
        ctrl_point_values: &[T],
        extrapolate_ends: bool,
    ) -> Vec<T>
    where
        T: std::ops::Add<T, Output = T>
            + std::ops::Sub<T, Output = T>
            + std::ops::Mul<f64, Output = T>
            + Copy,
    {
        let mut span_point_values: Vec<T> =
            Vec::with_capacity(self.nr_span_lines() + self.nr_wings());

        for wing_index in 0..self.wing_indices.len() {
            let first_index = self.wing_indices[wing_index].start;

            // First point is extrapolated
            if extrapolate_ends {
                let first_delta =
                    ctrl_point_values[first_index] - ctrl_point_values[first_index + 1];

                span_point_values.push(ctrl_point_values[first_index] + first_delta);
            } else {
                span_point_values.push(ctrl_point_values[first_index]);
            }

            // Loop over all span lines in the wing
            for i in self.wing_indices[wing_index].clone() {
                let last_index = self.wing_indices[wing_index].clone().last().unwrap();

                // Last point is extrapolated, all others are interpolated
                if i == last_index {
                    if extrapolate_ends {
                        let last_delta =
                            ctrl_point_values[last_index] - ctrl_point_values[last_index - 1];
                        span_point_values.push(ctrl_point_values[last_index] + last_delta);
                    } else {
                        span_point_values.push(ctrl_point_values[last_index]);
                    }
                } else {
                    span_point_values.push((ctrl_point_values[i] + ctrl_point_values[i + 1]) * 0.5);
                }
            }
        }

        span_point_values
    }
}
