// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality to calculate lift-induced velocity under the assumption that both the wings and 
//! the wake is steady. 
//! 
//! Using this assumption, the induced velocity is only dependent on the vortex strength, and can
//! therefore be reduced to a linear system of equations. Using this system is faster than
//! calculating the influence from individual vortex lines.

use serde::{Serialize, Deserialize};

use ndarray::prelude::*;

use crate::vec3::Vec3;
use crate::line_force_model::LineForceModel;
use crate::line_force_model::velocity_input::freestream::Freestream;

use crate::lifting_line::singularity_elements::prelude::*;

use super::velocity_corrections::{VelocityCorrections, VelocityCorrectionsBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteadyWakeBuilder {
    #[serde(default="SteadyWakeBuilder::default_wake_length_factor")]
    pub wake_length_factor: f64,
    #[serde(default)]
    pub symmetry_condition: SymmetryCondition,
    #[serde(default)]
    pub viscous_core_length: ViscousCoreLength,
    #[serde(default)]
    pub induced_velocity_corrections: VelocityCorrectionsBuilder
}

impl SteadyWakeBuilder {
    pub fn default_wake_length_factor() -> f64 {100.0}

    /// Constructs a matrix that can compute induced velocities as a function of the strength of 
    /// each vortex line, assuming that the strength is constant along the length of the vortex 
    /// line.
    ///
    /// # Arguments
    /// * `line_force_model` - an instance of a line force model to that is used to determine the
    /// geometry of the bound vortices fo the wake.
    /// * `freestream` - an instance of a freestream velocity model that is used to determine the
    /// wake direction.
    pub fn build(
        &self, 
        line_force_model: &LineForceModel, 
        freestream: &Freestream, 
    ) -> SteadyWake {
        let mean_chord_length: f64 = line_force_model.chord_vectors().iter().map(
            |chord| chord.length()
        ).sum::<f64>() / line_force_model.nr_span_lines() as f64;

        let ctrl_points = line_force_model.ctrl_points();

        let freestream_velocity = freestream.velocity_at_locations(
            &ctrl_points
        );

        let average_freestream_velocity: Vec3 = freestream_velocity.iter().sum::<Vec3>() / freestream_velocity.len() as f64;

        let wake_length = self.wake_length_factor * mean_chord_length;

        let wake_line_vector = average_freestream_velocity * wake_length;
        
        let span_lines = line_force_model.span_lines();

        let nr_span_lines = span_lines.len();

        let potential_theory_model = PotentialTheoryModel {
            viscous_core_length: self.viscous_core_length.clone(),
            symmetry_condition: self.symmetry_condition.clone(),
            ..Default::default()
        };

        let mut velocity_factors: Array2<Vec3> = Array2::from_elem(
            (nr_span_lines, nr_span_lines), Vec3::default()
        );

        // For each control point ...
        for i_row in 0..nr_span_lines {
            let ctrl_point = ctrl_points[i_row];

            // Calculate the induced velocity from each horseshoe vortex
            for i_col in 0..nr_span_lines {
                velocity_factors[[i_row, i_col]] = 
                    potential_theory_model.induced_velocity_from_horseshoe_with_unit_strength(
                        ctrl_point, 
                        &span_lines[i_col], 
                        wake_line_vector, 
                    );
            }
        }

        let induced_velocity_corrections = self.induced_velocity_corrections.build(average_freestream_velocity);

        SteadyWake {
            velocity_factors,
            induced_velocity_corrections
        }
    }
}

impl Default for SteadyWakeBuilder {
    fn default() -> Self {
        SteadyWakeBuilder {
            wake_length_factor: Self::default_wake_length_factor(),
            symmetry_condition: Default::default(),
            viscous_core_length: Default::default(),
            induced_velocity_corrections: Default::default()
        }
    }
}


#[derive(Debug, Clone)]
/// Data used to calculate the velocity at the ctrl points of the span lines, as a function of 
/// strength
pub struct SteadyWake {
    /// The factors that are multiplied with the strength of each vortex line to calculate the 
    /// lift-induced velocity at each control point
    pub velocity_factors: Array2<Vec3>,
    /// Optional corrections to the induced velocity
    pub induced_velocity_corrections: VelocityCorrections,
}

impl SteadyWake {
    /// Calculated the induced velocity at each control point.
    ///
    /// # Argument
    /// * `strength` - the strength for each vortex line in the line force model that the wake is
    /// built with.
    pub fn induced_velocities_at_control_points(&self, strength: &[f64]) -> Vec<Vec3> {
        let (nr_rows, nr_cols) = self.velocity_factors.dim();

        let mut induced_velocities: Vec<Vec3> = vec![Vec3::default(); nr_rows];

        for i_row in 0..nr_rows {
            for i_col in 0..nr_cols {
                induced_velocities[i_row] += 
                    self.velocity_factors[[i_row, i_col]] * strength[i_col];
            }
        }

        if self.induced_velocity_corrections.any_active_corrections() {
            self.induced_velocity_corrections.correct(&mut induced_velocities);
        }

        induced_velocities
    }
}