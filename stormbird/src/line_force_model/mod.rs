// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality for representing wings as "line objects", without any assumptions about how 
//! lift-induced velocities are estimated. In other words, this part is common for all methods 
//! available in the library, and therefore the foundation of all simulations. 

pub mod solver_utils;
pub mod span_line;
pub mod builder;
pub mod prescribed_circulations;
pub mod prelude;

use std::ops::Range;

use crate::math_utils::statistics::mean;

use crate::io_structs::prelude::*;
use crate::vec3::Vec3;
use crate::section_models::SectionModel;
use span_line::*;
use prescribed_circulations::PrescribedCirculation;

/// Input struct to add a single wing to a line force model
pub struct SingleWing {
    pub span_lines_local: Vec<SpanLine>,
    pub chord_vectors_local: Vec<Vec3>,
    pub section_model: SectionModel,
}

#[derive(Clone, Debug)]
/// The struct holds variables for a model that calculate the forces on wings, under the assumption
/// that they can be represented as a set of line elements. The intended use is in lifting line and 
/// actuator line simulations.
pub struct LineForceModel {
    /// Vector of line segments that defines the span geometry of the wings. Each have its own start 
    /// and end point, to allow for uncoupled analysis
    pub span_lines_local:     Vec<SpanLine>,
    /// Vectors representing both the chord length and the direction of the chord for each span line
    pub chord_vectors_local:  Vec<Vec3>,
    /// Two dimensional models for lift and drag coefficients for each wing in the model
    pub section_models: Vec<SectionModel>,
    /// Indices used to sort different wings from each other.
    pub wing_indices:   Vec<Range<usize>>,
    /// Translation from local to global coordinates
    pub translation: Vec3,
    /// Rotation from local to global coordinates
    pub rotation: Vec3,
    /// Vector used to store local angles for each wing. This can be used to rotate the wing along 
    /// the span axis during a dynamic simulation. The typical example is changing the angle of 
    /// attack on a wing sail due to changing apparent wind conditions.
    pub local_wing_angles: Vec<f64>,
    /// Density used in force calculations
    pub density: f64,
    /// Optional prescribed circulation shape
    pub prescribed_circulation: Option<PrescribedCirculation>,
}

impl LineForceModel {
    pub fn default_density() -> f64 { 1.225 }

    /// Creates a new empty line force model. Wings can be added using the [LineForceModel::add_wing] function.
    pub fn new(density: f64) -> LineForceModel {
        Self{
            span_lines_local: Vec::new(),
            chord_vectors_local:  Vec::new(),
            section_models: Vec::new(),
            wing_indices:  Vec::new(),
            translation: Vec3::default(),
            rotation: Vec3::default(),
            local_wing_angles: Vec::new(),
            density,
            prescribed_circulation: None,
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

        self.wing_indices.push( Range {start: start_index, end: end_index} );
        
        for line in &wing.span_lines_local {
            self.span_lines_local.push(line.clone());
        }
        
        for chord_vector in &wing.chord_vectors_local {
            self.chord_vectors_local.push(*chord_vector);
        }

        self.section_models.push(wing.section_model.clone());

        self.local_wing_angles.push(0.0);
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
    pub fn wing_rotation_axis(&self, wing_index: usize) -> Vec3 {
        self.span_lines_local[self.wing_indices[wing_index].start].relative_vector()
    }

    pub fn wing_rotation_axis_from_global(&self, global_index: usize) -> Vec3 {
        let wing_index = self.wing_index_from_global(global_index);
        
        self.wing_rotation_axis(wing_index)
    }

    /// Returns both angle and axis of rotation for the wing at the input index.
    pub fn wing_rotation_data(&self, wing_index: usize) -> (f64, Vec3) {
        let axis = self.wing_rotation_axis(wing_index);
        let angle = self.local_wing_angles[wing_index];

        (angle, axis)
    }

    pub fn wing_rotation_data_from_global(&self, global_index: usize) -> (f64, Vec3) {
        let wing_index = self.wing_index_from_global(global_index);
        
        self.wing_rotation_data(wing_index)
    }

    /// Resets the local wing angles to zero.
    pub fn reset_local_wing_angles(&mut self) {
        for angle in self.local_wing_angles.iter_mut() {
            *angle = 0.0;
        }
    }

    /// Returns the span lines in global coordinates.
    pub fn span_lines(&self) -> Vec<SpanLine> {
        self.span_lines_local.iter().enumerate().map(
            |(global_index, line)| {
                let (angle, axis) = self.wing_rotation_data_from_global(global_index);

                line.rotate_around_axis(angle, axis)
                    .rotate(self.rotation)
                    .translate(self.translation)
            }
        ).collect()
    }

    /// Returns the chord vectors in global coordinates.
    pub fn chord_vectors(&self) -> Vec<Vec3> {
        self.chord_vectors_local.iter().enumerate().map(
            |(global_index, chord_vector)| {
                let (angle, axis) = self.wing_rotation_data_from_global(global_index);

                chord_vector.rotate_around_axis(angle, axis).rotate(self.rotation)
            }
        ).collect()
    }

    /// Returns the control points of each line element. This is calculated as the midpoint of each
    /// span line
    pub fn ctrl_points(&self) -> Vec<Vec3> {
        self.span_lines().iter().map(|line| line.ctrl_point()).collect()
    }

    /// Returns the control points of each line element in local coordinates. This is calculated as
    /// the midpoint of each span line
    pub fn ctrl_points_local(&self) -> Vec<Vec3> {
        self.span_lines_local.iter().map(|line| line.ctrl_point()).collect()
    }    

    /// Returns the points making up the line geometry of the wings as a vector of spatial vectors,
    /// as opposed to a vector of span lines.
    pub fn span_points(&self) -> Vec<Vec3> {
        let span_lines = self.span_lines();
        let mut span_points: Vec<Vec3> = Vec::new();

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
    pub fn remove_span_velocity(&self, velocity: &[Vec3]) -> Vec<Vec3> {
        let span_lines = self.span_lines();

        velocity.iter().zip(span_lines.iter()).map(
            |(vel, line)| {
                let span_velocity = vel.project(line.relative_vector());

                *vel - span_velocity
            }
        ).collect()  
    }

    /// Return the angle of attack at each control point.
    /// 
    /// The angle is defined as the rotation from the chord vector to the velocity vector, using the 
    /// span line as the axis of rotation, with right handed positive rotation.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn angle_of_attack(&self, velocity: &[Vec3]) -> Vec<f64> {
        let chord_vectors = self.chord_vectors();
        let span_lines    = self.span_lines();
        
        (0..velocity.len()).map(|index| {
            chord_vectors[index].signed_angle_between(
                velocity[index], 
                span_lines[index].direction()
            )
        }).collect()
    }

    /// Returns the circulation strength, either directly or based on the prescribed shape, 
    /// depending on the fields in self.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn circulation_strength(&self, velocity: &[Vec3]) -> Vec<f64> {
        if self.prescribed_circulation.is_some() {
            self.prescribed_circulation_strength(velocity)
        } else {
            self.circulation_strength_raw(velocity)
        }
    }

    /// Returns the circulation strength on each line based on the lifting line equation.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn circulation_strength_raw(&self, velocity: &[Vec3]) -> Vec<f64> {
        let cl = self.lift_coefficients(&velocity);

        (0..velocity.len()).map(|index| {
            -0.5 * self.chord_vectors_local[index].length() * velocity[index].length() * cl[index] * self.density
        }).collect()
    }

    /// Returns the local lift coefficient on each line element.
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn lift_coefficients(&self, velocity: &[Vec3]) -> Vec<f64> {
        let angle_of_attack = self.angle_of_attack(velocity);

        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index  = self.wing_index_from_global(index);

                match &self.section_models[wing_index] {
                    SectionModel::Foil(foil) => 
                        foil.lift_coefficient(angle_of_attack[index]),
                    SectionModel::VaryingFoil(foil) => 
                        foil.lift_coefficient(angle_of_attack[index]),
                    SectionModel::RotatingCylinder(cylinder) => 
                        cylinder.lift_coefficient(
                            self.chord_vectors_local[index].length(), velocity[index].length()
                        ),
                }
            }
        ).collect()
    }

    /// Returns the viscous drag coefficient on each line element, based on the section model
    /// and the input velocity. 
    ///
    /// # Argument
    /// * `velocity` - the velocity vector at each control point
    pub fn viscous_drag_coefficients(&self, velocity: &[Vec3]) -> Vec<f64> {
        let angle_of_attack = self.angle_of_attack(velocity);

        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index  = self.wing_index_from_global(index);

                match &self.section_models[wing_index] {
                    SectionModel::Foil(foil) => 
                        foil.drag_coefficient(angle_of_attack[index]),
                    SectionModel::VaryingFoil(foil) => 
                        foil.drag_coefficient(angle_of_attack[index]),
                    SectionModel::RotatingCylinder(cylinder) => 
                        cylinder.drag_coefficient(self.chord_vectors_local[index].length(), velocity[index].length())
                }
            }
        ).collect()
    }

    /// Calculates the wake angle behind each line element.
    pub fn wake_angles(&self, velocity: &[Vec3]) -> Vec<f64> {
        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index  = self.wing_index_from_global(index);

                match &self.section_models[wing_index] {
                    SectionModel::Foil(_) => 0.0,
                    SectionModel::VaryingFoil(_) => 0.0,
                    SectionModel::RotatingCylinder(cylinder) => 
                        cylinder.wake_angle(
                            self.chord_vectors_local[index].length(), velocity[index].length()
                        )
                }
            }
        ).collect()
    }


    /// Calculates the average viscous drag coefficient for each wing.
    pub fn average_viscous_drag_coefficients(&self, velocity: &[Vec3]) -> Vec<f64> {
        let cd = self.viscous_drag_coefficients(velocity);

        self.wing_averaged_values(&cd)
    }
    
    /// Calculates the forces on each line element.
    pub fn sectional_forces(&self, input: &SectionalForcesInput) -> SectionalForces {
        let mut sectional_forces = SectionalForces {
            circulatory: self.sectional_circulatory_forces(&input.circulation_strength, &input.velocity),
            sectional_drag: self.sectional_drag_forces(&input.velocity),
            added_mass: self.sectional_added_mass_force(&input.acceleration),
            gyroscopic: self.sectional_gyroscopic_force(&input.velocity),
            total: vec![Vec3::default(); self.nr_span_lines()],
        };

        sectional_forces.compute_total();

        sectional_forces
    }

    /// Calculates the forces on each line element due to the circulatory forces (i.e., sectional lift)
    pub fn sectional_circulatory_forces(&self, strength: &[f64], velocity: &[Vec3]) -> Vec<Vec3> {
        let span_lines = self.span_lines();

        (0..self.nr_span_lines()).map(
            |index| {
                if velocity[index].length() == 0.0 {
                    Vec3::default()
                } else {
                    strength[index] * velocity[index].cross(span_lines[index].relative_vector())
                }
            }
        ).collect()
    }

    /// Calculates the forces on each line element due to the sectional drag model. This is most 
    /// often the visocus drag, but it can also include other physical effects if that is included
    /// in the sectional drag model.
    pub fn sectional_drag_forces(&self, velocity: &[Vec3]) -> Vec<Vec3> {
        let span_lines = self.span_lines();
        let cd = self.viscous_drag_coefficients(velocity);

        (0..self.nr_span_lines()).map(
            |index| {
                let drag_direction = velocity[index].normalize();

                let drag_area = self.chord_vectors_local[index].length() * span_lines[index].length();

                let force_factor = 0.5 * drag_area * self.density * velocity[index].length().powi(2);

                drag_direction * cd[index] * force_factor
            }
        ).collect()
    }

    /// Calculates the added mass force on each line element due to the flow acceleration at each 
    /// control point. 
    /// 
    /// **Note**: At the moment, this function only calculates the added mass due to the point 
    /// acceleration. However, according to, for instance, Theodorsen, the added mass should also 
    /// depend on the angular velocity and angular acceleration of the wing. Although these effects
    /// are expected to be small, it should be included in the future. This would, however, require
    /// more information about the motion of the wing to be included as arguments.
    /// 
    /// # Argument
    /// * `acceleration` - the acceleration of the flow at each control point. That is, if the only
    /// velocity is due to the motion of the wings, the acceleration will be opposite to the motion
    /// of the wings.
    pub fn sectional_added_mass_force(&self, acceleration: &[Vec3]) -> Vec<Vec3> {
        let span_lines = self.span_lines();
        let chord_vectors = self.chord_vectors();
        
        (0..self.nr_span_lines()).map(
            |index| {
                let wing_index  = self.wing_index_from_global(index);

                let strip_area = chord_vectors[index].length() * span_lines[index].length();

                let mut relevant_acceleration = acceleration[index];

                relevant_acceleration -= relevant_acceleration.project(span_lines[index].direction());

                match self.section_models[wing_index] {
                    SectionModel::Foil(_) | SectionModel::VaryingFoil(_) => {
                        relevant_acceleration -= relevant_acceleration.project(chord_vectors[index]);
                    },
                    _ => {}
                }

                let added_mass_coefficient = match &self.section_models[wing_index] {
                    SectionModel::Foil(foil) => {
                        foil.added_mass_coefficient(relevant_acceleration.length())
                    },
                    SectionModel::VaryingFoil(foil) => {
                        foil.added_mass_coefficient(relevant_acceleration.length())
                    },
                    SectionModel::RotatingCylinder(cylinder) => {
                        cylinder.added_mass_coefficient(relevant_acceleration.length())
                    }
                };

                added_mass_coefficient * self.density * strip_area * relevant_acceleration.normalize()
            }
        ).collect()
    }

    /// Calculates the gyroscopic force on each line element. This is only relevant for rotor sails.
    /// 
    /// Uses a simplfied approach where the rotational speed of the rotor is assumed to be 
    /// significantly larger than the rotational velocity of the sail, for instance due to roll or
    /// pitch motion of the boat.
    pub fn sectional_gyroscopic_force(&self, velocity: &[Vec3]) -> Vec<Vec3> {
        vec![Vec3::default(); self.nr_span_lines()]
    }

    /// Calculates the relative distance from the center off each wing for each control point.
    /// The absolute values are divided with the span of each wing. In other words, the 
    /// return value will vary between -0.5 and 0.5, where 0 is the center of the wing.
    pub fn relative_span_distance(&self) -> Vec<f64> {
        let mut relative_span_distance: Vec<f64> = Vec::new();

        for wing_index in 0..self.wing_indices.len() {

            let start_point = self.span_lines_local[
                self.wing_indices[wing_index].start
            ].start_point;
            
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

            let end_point = self.span_lines_local[
                self.wing_indices[wing_index].clone().last().unwrap()
            ].end_point;

            let total_distance = current_wing_span_distance.last().unwrap() + end_point.distance(previous_point);

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

    /// Shorthand for quickly calculating the typical force factor used when presenting 
    /// non-dimensional forces from a simulation (i.e., lift and drag coefficients)
    pub fn total_force_factor(&self, freestream_velocity: f64) -> f64 {
        0.5 * self.density * freestream_velocity.powi(2) * self.total_projected_area()
    }
    

    /// General function for calculating wing-averaged values
    pub fn wing_averaged_values<T>(&self, sectional_values: &[T]) -> Vec<T> 
    where T: 
        std::ops::Div<f64, Output = T> + 
        std::ops::Add<T, Output = T> + 
        Copy
    {
        let mut result: Vec<T> = Vec::new();

        for wing_indices in &self.wing_indices {
            result.push(
                mean(&sectional_values[wing_indices.clone()])
            );
        }

        result
    }

    /// Maps a constant value to a vector of values for each section
    pub fn section_values_from_constant_value<T>(&self, value: &T) -> Vec<T> 
    where T: Clone
    {
        let mut result: Vec<T> = Vec::new();

        for _ in 0..self.nr_span_lines() {
            result.push(value.clone());
        }

        result
    }

    /// Maps a vector of values for each wing to a vector of values for each section
    pub fn section_values_from_wing_values<T>(&self, wing_values: &[T]) -> Vec<T> 
    where T: Clone
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
    pub fn span_point_values_from_ctrl_point_values<T>(&self, ctrl_point_values: &[T], extrapolate_ends: bool) -> Vec<T> 
    where T: 
        std::ops::Add<T, Output = T> + 
        std::ops::Sub<T, Output = T> +
        std::ops::Mul<f64, Output = T> +
        Copy
    {
        let mut span_point_values: Vec<T> = Vec::with_capacity(self.nr_span_lines() + self.nr_wings());

        for wing_index in 0..self.wing_indices.len() {
            let first_index = self.wing_indices[wing_index].start;

            // First point is extrapolated
            if extrapolate_ends {
                let first_delta = ctrl_point_values[first_index] - ctrl_point_values[first_index + 1];
            
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
                        let last_delta = ctrl_point_values[last_index] - ctrl_point_values[last_index - 1];
                        span_point_values.push(ctrl_point_values[last_index] + last_delta);
                    } else {
                        span_point_values.push(ctrl_point_values[last_index]);
                    }
                } else {
                    span_point_values.push(
                        (ctrl_point_values[i] + ctrl_point_values[i+1]) * 0.5
                    );
                }  
            }
        }

        span_point_values
    }
    
}

impl Default for LineForceModel {
    fn default() -> Self {
        Self::new(Self::default_density())
    }
}