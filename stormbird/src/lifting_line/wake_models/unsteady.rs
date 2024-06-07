// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality for calculating lift-induced velocities from full dynamic wake.

use std::fs::File;
use std::io::{Write, BufWriter, Error};

use serde::{Serialize, Deserialize};

use rayon::prelude::*;
use rayon::iter::ParallelIterator;

use std::ops::Range;

use crate::vec3::Vec3;

use crate::line_force_model::LineForceModel;

use crate::lifting_line::singularity_elements::prelude::*;

use super::velocity_corrections::{VelocityCorrections, VelocityCorrectionsBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Enum to choose how to set the length of the wake. 
/// 
/// # Variants
/// * `NrPanels` - The wake length is determined by the number of panels in the wake. This makes it
/// independent of the freestream velocity and the mean chord length.
/// * `TargetLengthFactor` - The wake length is determined by the freestream velocity and the mean
/// chord length, multiplied by the given factor. This variant can only be used safely when the 
/// freestream velocity is properly defined when initializing the wake. This is not always the case, 
/// and the `NrPanels` variant is therefore the default.
pub enum WakeLength {
    NrPanels(usize),
    TargetLengthFactor(f64),
}

impl Default for WakeLength {
    fn default() -> Self {
        Self::NrPanels(200)
    }
}

impl WakeLength {
    fn nr_wake_panels_from_target_length_factor(
        &self, 
        chord_length: f64, 
        velocity: f64, 
        time_step: f64
    ) -> Result<usize, String> {
        match self {
            Self::NrPanels(_) => Err(
                "This function is only intended for the TargetLengthFactor variant".to_string()
            ),
            Self::TargetLengthFactor(factor) => Ok(
                (factor * chord_length / (velocity * time_step)).ceil() as usize
            )
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Variables used to build an unsteady wake. 
pub struct UnsteadyWakeBuilder {
    #[serde(default)]
    /// Data used to determine the length of the wake. 
    pub wake_length: WakeLength,
    
    #[serde(default)]
    /// The viscous core length used when calculating the induced velocities
    pub viscous_core_length: ViscousCoreLength,
    #[serde(default="UnsteadyWakeBuilder::default_first_panel_relative_length")]
    /// How the first panel in the wake is treated
    pub first_panel_relative_length: f64,
    #[serde(default="UnsteadyWakeBuilder::default_last_panel_relative_length")]
    /// Factor used to calculate the length of the final panel, relative to the chord length.
    pub last_panel_relative_length: f64,
    #[serde(default="UnsteadyWakeBuilder::default_strength_damping_last_panel_ratio")]
    /// Determines the damping factor for the wake strength. Specifies how much damping there should
    /// be on the last panel. The actual damping factor also depends on the number of wake panels.
    pub strength_damping_last_panel_ratio: f64,
    #[serde(default)]
    /// Symmetry condition
    pub symmetry_condition: SymmetryCondition,
    #[serde(default)]
    /// Ratio that determines how much of the wake points that are affected by the induced 
    /// velocities. A value lower than 1.0 can be used to speed up a simulation.
    pub ratio_of_wake_affected_by_induced_velocities: Option<f64>,
    #[serde(default="PotentialTheoryModel::default_far_field_ratio")]
    /// Determines how far away from a panel it is necessary to be before the far field method is
    /// used to calculate the induced velocity, rather than the full method.
    pub far_field_ratio: f64,
    #[serde(default)]
    /// Damping factor for the shape of the wake. A value of 0.0 means no damping (the wake moves
    /// freely), while a value of 1.0 means that the wake points are fixed in space.
    pub shape_damping_factor: f64,
    #[serde(default)]
    /// Optional corrections for the calculated induced velocity.
    pub induced_velocity_corrections: VelocityCorrectionsBuilder,
    #[serde(default)]
    /// Optional viscous core to be used when calculating induced velocities off body.
    pub viscous_core_length_off_body: Option<ViscousCoreLength>,
    #[serde(default)]
    /// Option to neglect the induced velocities on a wing from the wake of the same wing. This is 
    /// useful if the effect of self-induced velocities on lift and drag is calculated in another 
    /// way, for example with CFD, and the only reason for running lifting-line simulations is to 
    /// calculate the wing-to-wing interaction.
    /// 
    /// **WARNING**: should probably always be used in combination with a prescribed circulation 
    /// shape in the line force model to maintain a realistic local shape.
    pub neglect_self_induced_velocities: bool
}

impl UnsteadyWakeBuilder {
    fn default_strength_damping_last_panel_ratio() -> f64 {1.0}
    fn default_first_panel_relative_length() -> f64 {0.75}
    fn default_last_panel_relative_length() -> f64 {10.0}

    pub fn build(
        &self,
        time_step: f64, 
        line_force_model: &LineForceModel, 
        initial_velocity: Vec3, 
    ) -> UnsteadyWake {                
        let span_points   = line_force_model.span_points();
        let chord_vectors = line_force_model.chord_vectors();

        let span_points_chord_vectors = line_force_model.span_point_values_from_ctrl_point_values(
            &chord_vectors, true
        );
        
        let nr_wake_panels_along_span = line_force_model.nr_span_lines();
        let nr_wake_points_along_span = span_points.len();

        let mean_chord_length: f64 = chord_vectors.iter()
            .map(|chord| chord.length())
            .sum::<f64>() / chord_vectors.len() as f64;
        
        let nr_wake_panels_per_line_element = match self.wake_length {
            WakeLength::NrPanels(nr_panels) => nr_panels,
            WakeLength::TargetLengthFactor(_) => {
                if initial_velocity.length() == 0.0 {
                    panic!("Freestream velocity is zero. Cannot calculate wake length.");
                }

                self.wake_length.nr_wake_panels_from_target_length_factor(
                    mean_chord_length, initial_velocity.length(), time_step
                ).unwrap()
            }
        };

        let nr_wake_points_per_line_element = nr_wake_panels_per_line_element + 1;

        let mut wake_points: Vec<Vec3> = Vec::with_capacity(
            nr_wake_points_per_line_element * nr_wake_points_along_span
        );

        let wake_building_velocity = if initial_velocity.length() == 0.0 {
            Vec3::new(1e-6, 1e-6, 1e-6)
        } else {
            initial_velocity
        };

        for i_stream in 0..nr_wake_points_per_line_element {
            for i_span in 0..span_points.len() {
                let start_point = span_points[i_span];

                if i_stream == 0 {
                    wake_points.push(start_point);
                } else {
                    wake_points.push(
                        start_point + 
                        self.first_panel_relative_length * span_points_chord_vectors[i_span] +
                        ((i_stream-1) as f64) * time_step * wake_building_velocity
                    );
                }
            }
        }

        let end_index_induced_velocities_on_wake = if let Some(ratio) = self.ratio_of_wake_affected_by_induced_velocities {
            Some((ratio * nr_wake_panels_per_line_element as f64).ceil() as usize)
        } else {
            None
        };

        let strength_damping_factor =self.strength_damping_factor(nr_wake_panels_per_line_element);

        let settings = UnsteadyWakeSettings {
            first_panel_relative_length: self.first_panel_relative_length,
            last_panel_relative_length: self.last_panel_relative_length,
            strength_damping_factor,
            nr_wake_points_along_span,
            nr_wake_panels_along_span,
            nr_wake_panels_per_line_element,
            end_index_induced_velocities_on_wake,
            shape_damping_factor: self.shape_damping_factor,
            neglect_self_induced_velocities: self.neglect_self_induced_velocities
        };

        let potential_theory_model = PotentialTheoryModel {
            viscous_core_length: self.viscous_core_length.clone(),
            symmetry_condition: self.symmetry_condition.clone(),
            far_field_ratio: self.far_field_ratio,
            viscous_core_length_off_body: self.viscous_core_length_off_body.clone(),
            ..Default::default()
        };

        let nr_panels = nr_wake_panels_per_line_element * nr_wake_panels_along_span;

        let strengths: Vec<f64> = vec![0.0; nr_panels];

        let panel_geometry: Vec<PanelGeometry> = vec![PanelGeometry::default(); nr_panels];

        let induced_velocity_corrections = self.induced_velocity_corrections.build(initial_velocity);

        let mut wake = UnsteadyWake {
            wake_points,
            strengths,
            panel_geometry,
            settings,
            potential_theory_model,
            wing_indices: line_force_model.wing_indices.clone(),
            number_of_time_steps_completed: 0,
            induced_velocity_corrections
        };

        wake.update_panel_geometry_from_wake_points();

        wake
    }

    fn strength_damping_factor(&self, nr_wake_panels_per_line_element: usize) -> f64 {
        let estimated_value = 1.0 - self.strength_damping_last_panel_ratio.powf(
            1.0 / (nr_wake_panels_per_line_element - 1) as f64
        );

        estimated_value.max(0.0).min(1.0)
    }
}

impl Default for UnsteadyWakeBuilder {
    fn default() -> Self {
        Self {
            wake_length: Default::default(),
            viscous_core_length: Default::default(),
            first_panel_relative_length: Self::default_first_panel_relative_length(),
            last_panel_relative_length: Self::default_last_panel_relative_length(),
            strength_damping_last_panel_ratio: Self::default_strength_damping_last_panel_ratio(),
            symmetry_condition: Default::default(),
            ratio_of_wake_affected_by_induced_velocities: None,
            far_field_ratio: PotentialTheoryModel::default_far_field_ratio(),
            shape_damping_factor: 0.0,
            induced_velocity_corrections: Default::default(),
            viscous_core_length_off_body: None,
            neglect_self_induced_velocities: false
        }
    }
}

#[derive(Debug, Clone)]
/// Settings for the unsteady wake
pub struct UnsteadyWakeSettings {
    pub first_panel_relative_length: f64,
    pub last_panel_relative_length: f64,
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
    pub wake_points: Vec<Vec3>,
    /// The strengths of the vortex lines
    pub strengths: Vec<f64>,
    /// Panel geometry data used to determine what method to use for calculating the induced 
    /// velocities, and in the far field methods for the same purpose
    panel_geometry: Vec<PanelGeometry>,
    /// Settings for the wake behavior
    settings: UnsteadyWakeSettings,
    /// The model used to calculate induced velocities from vortex lines
    potential_theory_model: PotentialTheoryModel,
    /// To determine which wing the wake points belong to. Copied directly from the line force model
    wing_indices: Vec<Range<usize>>,
    /// Counter to keep track of the number of time steps that have been completed
    number_of_time_steps_completed: usize,
    /// Corrections for the induced velocity, such as max magnitude and correction factor.
    /// 
    /// By default, this is not used. However, it can be used on cases where the simulation is known
    /// to create unstable and too large induced velocities. The original use case is for rotor 
    /// sails.
    induced_velocity_corrections: VelocityCorrections
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
    pub fn induced_velocities(&self, points: &[Vec3], off_body: bool) -> Vec<Vec3> {
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
    pub fn induced_velocities_from_first_panels(&self, points: &[Vec3], off_body: bool) -> Vec<Vec3> {
        self.induced_velocities_local(points, 0, self.settings.nr_wake_panels_along_span, off_body, self.settings.neglect_self_induced_velocities)
    }

    /// Calculates the induced velocities from all the panels in the free wake, neglecting the first 
    /// panels, at the input points. 
    ///
    /// # Arguments
    /// * `points` - The points at which the induced velocities are calculated
    /// * `off_body` - If the points are off body, the induced velocities **can** be calculated with 
    /// the off-body viscous core length in the potential theory model if it exists.
    pub fn induced_velocities_from_free_wake(&self, points: &[Vec3], off_body: bool) -> Vec<Vec3> {
        self.induced_velocities_local(
            points, 
            self.settings.nr_wake_panels_along_span, 
            self.strengths.len(),
            off_body,
            self.settings.neglect_self_induced_velocities
        )
    }

    /// Update the strength of the wake panels closest to the wing geometry
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
        ctrl_points_freestream: &[Vec3],
        wake_points_freestream: &[Vec3]
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
    fn update_panel_geometry_from_wake_points(&mut self) {
        for i in 0..self.strengths.len() {
            let (stream_index, span_index) = self.reverse_panel_index(i);

            let panel_points = self.panel_wake_points(stream_index, span_index);

            self.panel_geometry[i] = PanelGeometry::new(panel_points);
        }
    }

    /// Calculates induced velocities from the panels starting at start_index and ending at end_index
    fn induced_velocities_local(
        &self, 
        points: &[Vec3], 
        start_index: usize, 
        end_index: usize, 
        off_body: bool,
        neglect_self_induced: bool
    ) -> Vec<Vec3> {
        let mut induced_velocities: Vec<Vec3> = points.par_iter().enumerate().map(|(point_index, point)| {
            (start_index..end_index).into_iter().map(|i_panel| {
                if neglect_self_induced {
                    let (_stream_index, span_index) = self.reverse_panel_index(i_panel);

                    let wing_index_panel = self.wing_index(span_index);
                    let wing_index_point = self.wing_index(point_index);

                    if wing_index_panel == wing_index_point {
                        Vec3::default()
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
    /// 
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
    fn panel_wake_points(&self, panel_stream_index: usize, panel_span_index: usize) -> [Vec3; 4] {
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
        ctrl_points_freestream: &[Vec3]
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
        let u_i: Vec<Vec3> = self.induced_velocities(&ctrl_points, true);

        let mut ctrl_points_velocity: Vec<Vec3> = Vec::with_capacity(ctrl_points.len());

        for i in 0..ctrl_points.len() {
            ctrl_points_velocity.push(ctrl_points_freestream[i] + u_i[i]);
        }

        ctrl_points_velocity = line_force_model.remove_span_velocity(&ctrl_points_velocity);

        let angles_of_attack = line_force_model.angles_of_attack(&ctrl_points_velocity);

        let wake_angles     = line_force_model.wake_angles(&ctrl_points_velocity);

        // Compute a change vector based on ctrl point data
        let mut ctrl_points_change_vector: Vec<Vec3> = Vec::with_capacity(
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
            let wake_direction = if amount_of_flow_separation.abs() < 0.5 {
                chord_vectors[i].normalize()
            } else {
                let axis = span_lines[i].relative_vector().normalize();

                ctrl_points_velocity[i]
                    .normalize()
                    .rotate_around_axis(wake_angles[i], axis)
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
        wake_points_freestream: &[Vec3]
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
        ctrl_points_freestream: &[Vec3],
        wake_points_freestream: &[Vec3]
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
    pub fn velocity_at_wake_points(&self, wake_points_freestream: &[Vec3]) -> Vec<Vec3> {
        let mut velocity: Vec<Vec3> = wake_points_freestream.to_vec();

        let end_index: usize = if let Some(end_index) = self.settings.end_index_induced_velocities_on_wake {
            self.wake_point_index(end_index, 0).min(self.wake_points.len())
        } else {
            self.wake_points.len()
        };

        if end_index > 0 && self.number_of_time_steps_completed > 2 {
            let u_i_calc: Vec<Vec3> = self.induced_velocities(&self.wake_points[0..end_index], true);

            for i in 0..end_index {
                velocity[i] += u_i_calc[i];
            }
        }

        velocity
    }

    /// Stream all free wake points based on the Euler method.
    fn stream_free_wake_points(&mut self, time_step: f64, wake_points_freestream: &[Vec3]) {
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

    /// Calculates the induced velocity from a single panel at the input point
    fn induced_velocity_from_panel(&self, panel_index: usize, point: Vec3, off_body: bool) -> Vec3 {
        if self.strengths[panel_index] == 0.0 {
            Vec3::default()
        } else {
            let (stream_index, span_index) = self.reverse_panel_index(panel_index);

            let panel_points = self.panel_wake_points(stream_index, span_index);
    
            self.potential_theory_model.induced_velocity_from_panel_with_unit_strength(
                &panel_points, 
                &self.panel_geometry[panel_index], 
                point,
                off_body
            ) * self.strengths[panel_index]
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
                self.wake_points[i].x, 
                self.wake_points[i].y, 
                self.wake_points[i].z
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
                self.wake_points[i].x, 
                self.wake_points[i].y, 
                self.wake_points[i].z
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