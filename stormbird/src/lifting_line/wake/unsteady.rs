// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality for calculating lift-induced velocities from full dynamic wake.

use std::fs::File;
use std::io::{Write, BufWriter, Error};

use rayon::prelude::*;
use rayon::iter::ParallelIterator;

use std::ops::Range;

use math_utils::spatial_vector::SpatialVector;

use crate::line_force_model::LineForceModel;

use crate::lifting_line::singularity_elements::prelude::*;

use super::velocity_corrections::VelocityCorrections;

#[derive(Debug, Clone)]
/// Settings for the unsteady wake
pub struct UnsteadyWakeSettings {
    pub first_panel_relative_length: f64,
    pub last_panel_relative_length: f64,
    pub use_chord_direction: bool,
    pub strength_damping_factor: f64,
    pub nr_wake_points_along_span: usize,
    pub nr_wake_panels_along_span: usize,
    pub nr_wake_panels_per_line_element: usize,
    pub end_index_induced_velocities_on_wake: Option<usize>,
    pub shape_damping_factor: f64,
    pub neglect_self_induced_velocities: bool
}

#[derive(Debug, Clone)]
/// Model of an unsteady wake for lifting line simulations
/// 
/// The induced velocities are calculated from vortex panels and their strengths.
/// 
/// The wake points and panels are assumed to be organized as a structured surface where indices
/// are stream wise-major.
/// 
/// A typical use case is as follows:
/// - For each time step, the points in the wake lying exactly on the wing lines are updated to 
/// match the current wing geometry (which might have moved since the last time step)
/// - The strength of the first panel is then updated iteratively to solve the lifting line
/// equations. This happens in whatever solver that use this model. This model is used to calculate
/// the velocity as a function of the strength.
/// - When the strength for a time step is solved, the final velocity at the control points are 
/// calculated.
/// - Finally, the wake points stream downstream, based on the current velocity field and time step.
/// 
/// There are methods to update the strength and the shape of the vortex line for each time step in 
/// the simulation.
pub struct UnsteadyWake {
    /// The points making up the vortex wake
    pub wake_points: Vec<SpatialVector<3>>,
    /// The strengths of the vortex lines
    pub strengths: Vec<f64>,
    /// Panel geometry data used to determine what method to use for calculating the induced 
    /// velocities, and in the far field methods for the same purpose
    pub panel_geometry: Vec<PanelGeometry>,
    /// Settings for the wake behavior
    pub settings: UnsteadyWakeSettings,
    /// The model used to calculate induced velocities from vortex lines
    pub potential_theory_model: PotentialTheoryModel,
    /// To determine which wing the wake points belong to. Copied directly from the line force model
    pub wing_indices: Vec<Range<usize>>,
    /// Counter to keep track of the number of time steps that have been completed
    pub number_of_time_steps_completed: usize,
    /// Corrections for the induced velocity, such as max magnitude and correction factor.
    /// 
    /// By default, this is not used. However, it can be used on cases where the simulation is known
    /// to create unstable and too large induced velocities. The original use case is for rotor 
    /// sails.
    pub induced_velocity_corrections: VelocityCorrections
}

impl UnsteadyWake {
    /// Takes a line force vector as input, that might have a different position and orientation 
    /// than the current model, and updates the relevant internal geometry
    ///
    /// # Argument
    /// * `line_force_model` - The line force model that the wake is based on
    pub fn synchronize_wing_geometry(&mut self, line_force_model: &LineForceModel) {
        let span_points = line_force_model.span_points();

        for i in 0..span_points.len() {
            self.wake_points[i] = span_points[i];
        }

        self.update_panel_geometry_from_wake_points();
    }

    /// Calculates the induced velocities from all the panels in the wake
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// * `off_body` - If the points are off body, the induced velocities **can** be calculated with 
    /// the off-body viscous core length in the potential theory model if it exists.
    pub fn induced_velocities(&self, points: &[SpatialVector<3>], off_body: bool) -> Vec<SpatialVector<3>> {
        self.induced_velocities_local(points, 0, self.strengths.len(), off_body, false)
    }

    /// Calculates the induced velocity from the first panels in the stream wise direction only. This
    /// is used to calculate the velocity at the control points in the strength solver more 
    /// efficiently, as each iteration only updates the strength of these panels.
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// * `off_body` - If the points are off body, the induced velocities **can** be calculated with 
    /// the off-body viscous core length in the potential theory model if it exists.
    pub fn induced_velocities_from_first_panels(&self, points: &[SpatialVector<3>], off_body: bool) -> Vec<SpatialVector<3>> {
        self.induced_velocities_local(points, 0, self.settings.nr_wake_panels_along_span, off_body, self.settings.neglect_self_induced_velocities)
    }

    /// Calculates the induced velocities from all the panels in the free wake, neglecting the first 
    /// panels, at the input points. 
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// * `off_body` - If the points are off body, the induced velocities **can** be calculated with 
    /// the off-body viscous core length in the potential theory model if it exists.
    pub fn induced_velocities_from_free_wake(&self, points: &[SpatialVector<3>], off_body: bool) -> Vec<SpatialVector<3>> {
        self.induced_velocities_local(
            points, 
            self.settings.nr_wake_panels_along_span, 
            self.strengths.len(),
            off_body,
            self.settings.neglect_self_induced_velocities
        )
    }

    /// Update the strength of the wake panels closest to the wing geometry.
    /// 
    /// This is the same as updating the circulation strength on the first panels in the wake.
    pub fn update_wing_strength(&mut self, new_circulation_strength: &[f64]) {
        for i in 0..new_circulation_strength.len() {
            self.strengths[i] = new_circulation_strength[i];
        }
    }

    /// Update the wake geometry and strength based on the final solution at a time step.
    /// 
    /// This will:
    /// 1) stream the wake points downstream
    /// 2) stream the strength downstream
    pub fn update_after_completed_time_step(
        &mut self, 
        new_circulation_strength: &[f64], 
        time_step: f64, 
        line_force_model: &LineForceModel,
        ctrl_points_freestream: &[SpatialVector<3>],
        wake_points_freestream: &[SpatialVector<3>]
    ) {
        self.update_wake_points_after_completed_time_step(
            time_step, 
            line_force_model, 
            ctrl_points_freestream, 
            wake_points_freestream
        );
        self.update_panel_geometry_from_wake_points();
        self.update_strength_after_completed_time_step(new_circulation_strength);

        self.number_of_time_steps_completed += 1;
    }

    /// Update the panel geometry based on the current wake points
    pub fn update_panel_geometry_from_wake_points(&mut self) {
        for i in 0..self.strengths.len() {
            let (stream_index, span_index) = self.reverse_panel_index(i);

            let panel_points = self.panel_wake_points(stream_index, span_index);

            self.panel_geometry[i] = PanelGeometry::new(panel_points);
        }
    }

    /// Calculates induced velocities from the panels starting at start_index and ending at end_index
    pub fn induced_velocities_local(
        &self, 
        points: &[SpatialVector<3>], 
        start_index: usize, 
        end_index: usize, 
        off_body: bool,
        neglect_self_induced: bool
    ) -> Vec<SpatialVector<3>> {
        let mut induced_velocities: Vec<SpatialVector<3>> = points.par_iter().enumerate().map(|(point_index, point)| {
            (start_index..end_index).into_iter().map(|i_panel| {
                if neglect_self_induced {
                    let (_stream_index, span_index) = self.reverse_panel_index(i_panel);

                    let wing_index_panel = self.wing_index(span_index);
                    let wing_index_point = self.wing_index(point_index);

                    if wing_index_panel == wing_index_point {
                        SpatialVector::<3>::default()
                    } else {
                        self.induced_velocity_from_panel(i_panel, *point, off_body)
                    }

                } else {
                    self.induced_velocity_from_panel(i_panel, *point, off_body)
                }
            }).sum()
        }).collect();

        if self.induced_velocity_corrections.any_active_corrections() {
            self.induced_velocity_corrections.correct(&mut induced_velocities)
        }

        induced_velocities
    }


    #[inline(always)]
    /// Returns a flatten index for the wake panels. The panels are ordered streamwise-major.
    fn panel_index(&self, stream_index: usize, span_index: usize) -> usize {   
        stream_index * self.settings.nr_wake_panels_along_span + span_index
    }

    #[inline(always)]
    /// Returns the stream and span indices from a flatten index
    fn reverse_panel_index(&self, flat_index: usize) -> (usize, usize) {
        let stream_index = flat_index / self.settings.nr_wake_panels_along_span;
        let span_index   = flat_index % self.settings.nr_wake_panels_along_span;

        (stream_index, span_index)
    }

    #[inline(always)]
    /// Returns a flatten index for the wake points. The points are ordered streamwise-major.
    fn wake_point_index(&self, stream_index: usize, span_index: usize) -> usize {
        stream_index * self.settings.nr_wake_points_along_span + span_index
    }

    /// Returns the index of the wing that the span index belongs to
    fn wing_index(&self, span_index: usize) -> usize {
        for i in 0..self.wing_indices.len() {
            if self.wing_indices[i].contains(&span_index) {
                return i;
            }
        }

        panic!("Span index not found in any wing");
    }

    /// Returns the the indices to the four points that make up a panel at the given indices.
    /// 
    /// The indices are ordered in a counter-clockwise manner. The first index is for the bottom 
    /// left corner when viewing the panel from above.
    fn panel_wake_point_indices(&self, panel_stream_index: usize, panel_span_index: usize) -> [usize; 4] {
        let wing_index = self.wing_index(panel_span_index);
        
        [
            self.wake_point_index(panel_stream_index,     panel_span_index + wing_index),
            self.wake_point_index(panel_stream_index,     panel_span_index + 1 + wing_index),
            self.wake_point_index(panel_stream_index + 1, panel_span_index + 1 + wing_index),
            self.wake_point_index(panel_stream_index + 1, panel_span_index + wing_index),
        ]
    }

    /// Returns the four points that make up a panel at the given indices
    fn panel_wake_points(&self, panel_stream_index: usize, panel_span_index: usize) -> [SpatialVector<3>; 4] {
        let point_indices = self.panel_wake_point_indices(panel_stream_index, panel_span_index);

        [
            self.wake_points[point_indices[0]],
            self.wake_points[point_indices[1]],
            self.wake_points[point_indices[2]],
            self.wake_points[point_indices[3]],
        ]
    }

    /// Moves the first wake points after the wing geometry itself.
    /// 
    /// How the points are moved depends on both the sectional force model for each wing and - in 
    /// some cases - the angle of attack on each line force model.
    fn move_first_free_wake_points(
        &mut self, 
        line_force_model: &LineForceModel, 
        ctrl_points_freestream: &[SpatialVector<3>]
    ) {                
        assert!(
            line_force_model.nr_span_lines() == self.settings.nr_wake_panels_along_span, 
            "Number of span lines in line force model does not match number of wake points in wake model"
        );
        
        // Extract relevant information from the line force model
        let span_lines = line_force_model.span_lines();
        let chord_vectors = line_force_model.chord_vectors();
        let ctrl_points = line_force_model.ctrl_points();

        // Compute the induced velocities at the control points
        let u_i = if let Some(end_index) = self.settings.end_index_induced_velocities_on_wake {
            if end_index > 0 {
                self.induced_velocities(&ctrl_points, true)
            } else {
                vec![SpatialVector::<3>::default(); ctrl_points.len()]
            }
        } else {
            self.induced_velocities(&ctrl_points, true)
        };

        let mut ctrl_points_velocity: Vec<SpatialVector<3>> = Vec::with_capacity(ctrl_points.len());

        for i in 0..ctrl_points.len() {
            ctrl_points_velocity.push(ctrl_points_freestream[i] + u_i[i]);
        }

        let angles_of_attack = line_force_model.angles_of_attack(&ctrl_points_velocity);

        let wake_angles     = line_force_model.wake_angles(&ctrl_points_velocity);

        // Compute a change vector based on ctrl point data
        let mut ctrl_points_change_vector: Vec<SpatialVector<3>> = Vec::with_capacity(
            self.settings.nr_wake_panels_along_span
        );

        for i in 0..self.settings.nr_wake_panels_along_span {
            let wing_index = line_force_model.wing_index_from_global(i);

            let amount_of_flow_separation = line_force_model
                .section_models[wing_index]
                .amount_of_flow_separation(angles_of_attack[i]);
            
            // Little flow separation means that the ctrl point should move in the direction of the
            // chord vector. Large flow separation means that the ctrl point should move in the
            // direction of the velocity vector, but with an optional rotation around the axis of
            // the span line.
            let velocity_direction = ctrl_points_velocity[i].rotate_around_axis(
                wake_angles[i], 
                span_lines[i].relative_vector().normalize()
            ).normalize();

            let wake_direction = if self.settings.use_chord_direction {
                let chord_direction = chord_vectors[i].normalize();

                (
                    velocity_direction * amount_of_flow_separation + 
                    chord_direction * (1.0 - amount_of_flow_separation)
                ).normalize()
            } else {
                velocity_direction
            };

            ctrl_points_change_vector.push(
                self.settings.first_panel_relative_length * chord_vectors[i].length() * wake_direction
            );
        }

        // Transfer ctrl point data to span lines
        let span_points_change_vector = line_force_model.span_point_values_from_ctrl_point_values(
            &ctrl_points_change_vector, true
        );

        // Update the wake points
        let old_start_index = self.settings.nr_wake_points_along_span;
        let old_end_index   = 2 * self.settings.nr_wake_points_along_span;

        let old_wake_points = self.wake_points[old_start_index..old_end_index].to_vec();

        for i in 0..self.settings.nr_wake_points_along_span {
            let estimated_new_wake_point = self.wake_points[i] + span_points_change_vector[i];
            
            self.wake_points[i + self.settings.nr_wake_points_along_span] = 
                old_wake_points[i] * self.settings.shape_damping_factor + 
                estimated_new_wake_point * (1.0 - self.settings.shape_damping_factor);
        }
    }


    /// Moves the last points in the wake based on the chord length and the freestream velocity
    fn move_last_wake_points(
        &mut self,
        line_force_model: &LineForceModel,
        wake_points_freestream: &[SpatialVector<3>]
    ) {
        let start_index_last = self.wake_points.len() - self.settings.nr_wake_points_along_span;
        let start_index_previous = start_index_last - self.settings.nr_wake_points_along_span;

        let chord_vectors = line_force_model.span_point_values_from_ctrl_point_values(
            &line_force_model.chord_vectors(), true
        );

        for i in 0..self.settings.nr_wake_points_along_span {
            let current_velocity = wake_points_freestream[start_index_last + i];
            let change_vector = self.settings.last_panel_relative_length * chord_vectors[i].length() * current_velocity.normalize();

            self.wake_points[start_index_last + i] = self.wake_points[start_index_previous + i] + change_vector;
        }
    }

    /// Update the wake points by streaming them downstream.
    /// 
    /// The first and second "rows" - meaning the wing geometries and the first row of wake points -
    /// are treaded as special cases. The rest are moved based on the euler method
    fn update_wake_points_after_completed_time_step(
        &mut self, 
        time_step: f64,
        line_force_model: &LineForceModel,
        ctrl_points_freestream: &[SpatialVector<3>],
        wake_points_freestream: &[SpatialVector<3>]
    ) {
        self.move_first_free_wake_points(line_force_model, ctrl_points_freestream);
        self.stream_free_wake_points(time_step, wake_points_freestream);
        self.move_last_wake_points(line_force_model, wake_points_freestream);
    }

    /// Returns the velocity at all the wake points.
    ///
    /// The velocity is calculated as the sum of the freestream velocity and the induced velocity.
    /// However, if the settings contains and end-index for the induced velocities, the induced
    /// velocities can be neglected for the last panels. This is useful for speeding up simulations.
    ///
    /// # Argument
    /// * `freestream` - A model for the freestream velocity in the simulation
    pub fn velocity_at_wake_points(&self, wake_points_freestream: &[SpatialVector<3>]) -> Vec<SpatialVector<3>> {
        let mut velocity: Vec<SpatialVector<3>> = wake_points_freestream.to_vec();

        let end_index: usize = if let Some(end_index) = self.settings.end_index_induced_velocities_on_wake {
            self.wake_point_index(end_index, 0).min(self.wake_points.len())
        } else {
            self.wake_points.len()
        };

        if end_index > 0 && self.number_of_time_steps_completed > 2 {
            let u_i_calc: Vec<SpatialVector<3>> = self.induced_velocities(&self.wake_points[0..end_index], true);

            for i in 0..end_index {
                velocity[i] += u_i_calc[i];
            }
        }

        velocity
    }

    /// Stream all free wake points based on the Euler method.
    fn stream_free_wake_points(&mut self, time_step: f64, wake_points_freestream: &[SpatialVector<3>]) {
        let old_wake_points = self.wake_points.clone();

        let velocity = self.velocity_at_wake_points(wake_points_freestream);

        // Don't move the first panel. This is done in another function
        let start_index = 2 * self.settings.nr_wake_points_along_span;

        for i in start_index..self.wake_points.len() {
            let previous_wake_point = old_wake_points[i - self.settings.nr_wake_points_along_span];
            let previous_velocity   = velocity[i - self.settings.nr_wake_points_along_span];

            let integrated_point = previous_wake_point + time_step * previous_velocity;

            if self.settings.shape_damping_factor > 0.0 {
                let current_wake_point = self.wake_points[i];

                self.wake_points[i] = current_wake_point * self.settings.shape_damping_factor + 
                    integrated_point * (1.0 - self.settings.shape_damping_factor);
            } else {
                self.wake_points[i] = integrated_point;
            }
        }
    }

    /// Calculates the induced velocity from a single panel at the input point with unit strength
    pub fn unit_strength_induced_velocity_from_panel(
        &self, 
        stream_index: usize,
        span_index: usize,
        point: SpatialVector<3>, 
        off_body: bool
    ) -> SpatialVector<3> {
        let panel_index = self.panel_index(stream_index, span_index);
        let panel_points = self.panel_wake_points(stream_index, span_index);

        self.potential_theory_model.induced_velocity_from_panel_with_unit_strength(
            &panel_points, 
            &self.panel_geometry[panel_index], 
            point,
            off_body
        )
    }

    /// Calculates the induced velocity from a single panel at the input point with unit strength
    pub fn unit_strength_induced_velocity_from_panel_flat_index(
        &self, 
        panel_index: usize, 
        point: SpatialVector<3>, 
        off_body: bool
    ) -> SpatialVector<3> {
        let (stream_index, span_index) = self.reverse_panel_index(panel_index);

        self.unit_strength_induced_velocity_from_panel(stream_index, span_index, point, off_body)
    }

    /// Calculates the induced velocity from a single panel at the input point
    fn induced_velocity_from_panel(&self, panel_index: usize, point: SpatialVector<3>, off_body: bool) -> SpatialVector<3> {
        if self.strengths[panel_index] == 0.0 {
            SpatialVector::<3>::default()
        } else {
            let unit_velocity = self.unit_strength_induced_velocity_from_panel_flat_index(panel_index, point, off_body);

            self.strengths[panel_index] * unit_velocity
        }
    }

    /// Shift strength values downstream and update the wing values with the new circulation
    /// 
    /// Principle: the strength of each panel is updated to be the same as the previous panel in the
    /// stream wise direction in the last time step.
    ///
    /// # Argument
    /// * `new_circulation_strength` - The new circulation strength for the wing
    fn update_strength_after_completed_time_step(&mut self, new_circulation_strength: &[f64]) {
        let update_factor = 1.0 - self.settings.strength_damping_factor; // TODO: implement more sophisticated damping...

        let old_strengths = self.strengths.clone();

        for i_stream in 1..self.settings.nr_wake_panels_per_line_element {
            for i_span in 0..self.settings.nr_wake_panels_along_span {
                let current_index  = self.panel_index(i_stream, i_span);
                let previous_index = self.panel_index(i_stream - 1, i_span);

                self.strengths[current_index] = update_factor * old_strengths[previous_index];
            }
        }

        self.update_wing_strength(new_circulation_strength);
    }

    /// Export the wake geometry as an obj file
    ///
    /// # Argument
    /// * `file_path` - The path to the file to be written
    pub fn write_wake_to_obj_file(&self, file_path: &str) -> Result<(), Error> {
        let f = File::create(file_path)?;

        let mut writer = BufWriter::new(f);

        write!(writer, "o wake\n")?;

        for i in 0..self.wake_points.len(){
            write!(
                writer, 
                "v {} {} {}\n", 
                self.wake_points[i][0], 
                self.wake_points[i][1], 
                self.wake_points[i][2]
            )?;
        };

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.reverse_panel_index(panel_index);

            let indices = self.panel_wake_point_indices(stream_index, span_index);

            write!(
                writer, 
                "f {} {} {} {}\n", 
                indices[0] + 1, 
                indices[1] + 1, 
                indices[2] + 1, 
                indices[3] + 1
            )?;
        }

        writer.flush()?;

        Ok(())
    }

    /// Export the wake geometry and strength as a VTK file
    ///
    /// # Argument
    /// * `file_path` - The path to the file to be written
    pub fn write_wake_to_vtk_file(&self, file_path: &str) -> Result<(), Error> {
        let f = File::create(file_path)?;

        let mut writer = BufWriter::new(f);

        let nr_points = self.wake_points.len();
        let nr_faces  = self.strengths.len();

        // Header
        write!(writer, "<?xml version=\"1.0\"?>\n")?;
        write!(writer, "<VTKFile type=\"PolyData\" version=\"0.1\" byte_order=\"LittleEndian\">\n")?;
        write!(writer, "\t<PolyData>\n")?;
        write!(
            writer, 
            "\t\t<Piece NumberOfPoints=\"{}\" NumberOfVerts=\"0\" NumberOfLines=\"0\" NumberOfStrips=\"0\" NumberOfPolys=\"{}\">\n", 
            nr_points, 
            nr_faces
        )?;

        // Write points
        write!(writer, "\t\t\t<Points>\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Float32\" NumberOfComponents=\"3\" format=\"ascii\">\n")?;
        for i in 0..nr_points {
            write!(
                writer, 
                "\t\t\t\t\t{} {} {}\n", 
                self.wake_points[i][0], 
                self.wake_points[i][1], 
                self.wake_points[i][2]
            )?;
        }

        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t</Points>\n")?;

        // Write faces
        write!(writer, "\t\t\t<Polys>\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Int32\" Name=\"connectivity\" format=\"ascii\">\n")?;

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.reverse_panel_index(panel_index);

            let indices = self.panel_wake_point_indices(stream_index, span_index);

            write!(
                writer, 
                "\t\t\t\t\t{} {} {} {}\n", 
                indices[0], 
                indices[1], 
                indices[2], 
                indices[3]
            )?;
        }

        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Int32\" Name=\"offsets\" format=\"ascii\">\n")?;
        for i in 0..nr_faces {
            write!(writer, "\t\t\t\t\t{}\n", (i+1)*4)?;
        }
        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t</Polys>\n")?;

        // Write strength
        write!(writer, "\t\t\t<CellData Scalars=\"strength\">\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Float32\" Name=\"strength\" format=\"ascii\">\n")?;
        for i in 0..nr_faces {
            write!(writer, "\t\t\t\t\t{}\n", self.strengths[i])?;
        }

        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t</CellData>\n")?;

        write!(writer, "\t\t</Piece>\n")?;
        write!(writer, "\t</PolyData>\n")?;
        write!(writer, "</VTKFile>\n")?;

        writer.flush()?;

        Ok(())
    }
}