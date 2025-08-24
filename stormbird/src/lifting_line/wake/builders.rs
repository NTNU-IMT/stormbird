// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality for calculating lift-induced velocities from full dynamic wake.

use serde::{Deserialize, Serialize};

use stormath::{
    spatial_vector::SpatialVector,
    type_aliases::Float,
    consts::{PI, MIN_POSITIVE, INFINITY},
};

use crate::line_force_model::{
    LineForceModel,
    global_geometry_data::GlobalLineForceModelGeometry,
};

use crate::lifting_line::singularity_elements::prelude::*;

use super::{
    Wake,
    settings::{
        WakeSettings,
        WakeIndices,
    },
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Enum to represent different ways a viscous core length can be specified.
pub enum ViscousCoreLength {
    /// Signifies that the viscous core length is a fraction of the length of the vortex line. To
    /// be used, the vortex line geometry must be known.
    Relative(Float),
    /// Signifies that the viscous core length is an absolute value, and that it can be used without
    /// any more information about the geometry.
    Absolute(Float),
    /// Signifies that the viscous core length is not used.
    NoViscousCore,
}

impl Default for ViscousCoreLength {
    fn default() -> Self {
        Self::Relative(0.1)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SinIncreasedViscousCoreLength {
    pub last_panel_value: ViscousCoreLength,
    #[serde(default="SinIncreasedViscousCoreLength::default_evolution_length_factor")]
    pub evolution_length_factor: Float,
}

impl SinIncreasedViscousCoreLength {
    fn default_evolution_length_factor() -> Float { 1.0 }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ViscousCoreLengthEvolution {
    Constant,
    SinIncrease(SinIncreasedViscousCoreLength)
}

impl Default for ViscousCoreLengthEvolution {
    fn default() -> Self {
        Self::Constant
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Variables used to build a wake model.
pub struct WakeBuilder {
    #[serde(default="WakeBuilder::default_number_of_panels_per_line_element")]
    /// Number of panels.
    pub nr_panels_per_line_element: usize,
    #[serde(default)]
    /// The viscous core length used when calculating the induced velocities
    pub viscous_core_length: ViscousCoreLength,
    #[serde(default)]
    /// How the viscous core length should evolve behind the wake.
    pub viscous_core_length_evolution: ViscousCoreLengthEvolution,
    #[serde(default="WakeBuilder::default_first_panel_relative_length")]
    /// How the first panel in the wake is treated
    pub first_panel_relative_length: Float,
    #[serde(default="WakeBuilder::default_last_panel_relative_length")]
    /// Factor used to calculate the length of the final panel, relative to the chord length.
    pub last_panel_relative_length: Float,
    #[serde(default)]
    /// Determines if the chord direction should be used when calculating the direction of the first
    /// wake panels
    pub use_chord_direction: bool,
    #[serde(default)]
    /// Symmetry condition
    pub symmetry_condition: SymmetryCondition,
    #[serde(default)]
    /// Ratio that determines how much of the wake points that are affected by the induced
    /// velocities. The default is zero, which means no wake points are affected by the induced
    /// velocities. A value of 1.0 means that all wake points are affected by the induced
    /// velocities.
    pub ratio_of_wake_affected_by_induced_velocities: Float,
    #[serde(default="PotentialTheorySettings::default_far_field_ratio")]
    /// Determines how far away from a panel it is necessary to be before the far field method is
    /// used to calculate the induced velocity, rather than the full method.
    pub far_field_ratio: Float,
    #[serde(default)]
    /// Damping factor for the shape of the wake. A value of 0.0 means no damping (the wake moves
    /// freely), while a value of 1.0 means that the wake points are fixed in space.
    pub shape_damping_factor: Float,
    #[serde(default)]
    /// Option to neglect the induced velocities on a wing from the wake of the same wing. This is
    /// useful if the effect of self-induced velocities on lift and drag is calculated in another
    /// way, for example with CFD, and the only reason for running lifting-line simulations is to
    /// calculate the wing-to-wing interaction.
    ///
    /// **WARNING**: should probably always be used in combination with a prescribed circulation
    /// shape in the line force model to maintain a realistic local shape.
    pub neglect_self_induced_velocities: bool,
    #[serde(default="WakeBuilder::default_initial_relative_wake_length")]
    /// Length of wake during initialization, relative to the chord length
    pub initial_relative_wake_length: Float,
    #[serde(default)]
    /// A variable to determine whether the wake geometry and data should be written to a file
    pub write_wake_data_to_file: bool,
    #[serde(default)]
    /// The path to the folder where the wake data should be written to
    pub wake_files_folder_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Variables used to build a steady wake model.
pub struct SteadyWakeBuilder {
    #[serde(default="SteadyWakeBuilder::default_wake_length_factor")]
    pub wake_length_factor: Float,
    #[serde(default)]
    pub symmetry_condition: SymmetryCondition,
    #[serde(default)]
    pub viscous_core_length: ViscousCoreLength,
}


impl WakeBuilder {
    fn default_number_of_panels_per_line_element() -> usize {100}
    fn default_initial_relative_wake_length() -> Float {100.0}
    fn default_first_panel_relative_length() -> Float {0.75}
    fn default_last_panel_relative_length() -> Float {25.0}

    pub fn build(
        &self,
        line_force_model: &LineForceModel,
    ) -> Wake {
        let span_points = line_force_model.span_points();

        let indices = self.get_wake_indices(line_force_model);

        let mut points = vec![SpatialVector::default(); indices.nr_points()];

        for i_stream in 0..indices.nr_points_per_line_element {
            for i_span in 0..span_points.len() {
                let flat_index = indices.point_index(i_stream, i_span);

                points[flat_index] = span_points[i_span];
            }
        }

        let end_index_induced_velocities_on_wake = (
            self.ratio_of_wake_affected_by_induced_velocities *
            indices.nr_panels_per_line_element as Float
        ).ceil() as usize;

        let settings = WakeSettings {
            first_panel_relative_length: self.first_panel_relative_length,
            last_panel_relative_length: self.last_panel_relative_length,
            use_chord_direction: self.use_chord_direction,
            end_index_induced_velocities_on_wake,
            shape_damping_factor: self.shape_damping_factor,
            neglect_self_induced_velocities: self.neglect_self_induced_velocities,
            write_wake_data_to_file: self.write_wake_data_to_file,
            wake_files_folder_path: self.wake_files_folder_path.clone(),
        };

        let potential_theory_settings = PotentialTheorySettings {
            symmetry_condition: self.symmetry_condition.clone(),
            far_field_ratio: self.far_field_ratio,
            ..Default::default()
        };

        let nr_panels = indices.nr_panels();

        let strengths: Vec<Float> = vec![0.0; nr_panels];

        let panels_viscous_core_length = self.get_panels_viscous_core_length(
            line_force_model,
            &indices
        );

        let panels = vec![Panel::default(); nr_panels];

        let velocity_at_points = vec![SpatialVector::default(); indices.nr_points()];

        let representative_chord_length = line_force_model.chord_lengths.iter()
            .map(|c| *c)
            .sum::<Float>() / line_force_model.chord_lengths.len() as Float;

        let mut wake = Wake {
            indices,
            points,
            velocity_at_points,
            strengths,
            panels_viscous_core_length,
            settings,
            potential_theory_settings,
            wing_indices: line_force_model.wing_indices.clone(),
            number_of_time_steps_completed: 0,
            panels,
            representative_chord_length,
        };

        let line_force_model_geometry = GlobalLineForceModelGeometry::new(
            line_force_model
        );

        wake.initialize_based_on_chord_length(
            &line_force_model_geometry, 
            self.initial_relative_wake_length
        );

        wake
    }

    /// The function that calculates the viscous core length for each panel in the wake.
    pub fn get_panels_viscous_core_length(
        &self, 
        line_force_model: &LineForceModel,
        wake_indices: &WakeIndices,
    ) -> Vec<Float> {
        let mut out = vec![0.0; wake_indices.nr_panels()];

        let span_lines = line_force_model.span_lines();

        let span_line_lengths: Vec<Float> = span_lines.iter()
            .map(|line| line.length())
            .collect();

        for i_stream in 0..wake_indices.nr_panels_per_line_element {
            for i_span in 0..wake_indices.nr_panels_along_span {
                let flat_index = wake_indices.panel_index(
                    i_stream, 
                    i_span
                );

                let viscous_core_length_raw = match self.viscous_core_length {
                    ViscousCoreLength::Relative(relative_length) => {
                        relative_length * span_line_lengths[i_span]
                    },
                    ViscousCoreLength::Absolute(length) => length,
                    ViscousCoreLength::NoViscousCore => MIN_POSITIVE
                };

                out[flat_index] = match self.viscous_core_length_evolution {
                    ViscousCoreLengthEvolution::Constant => viscous_core_length_raw,
                    ViscousCoreLengthEvolution::SinIncrease(evolution_settings) => {
                        let final_viscous_core_length = match evolution_settings.last_panel_value {
                            ViscousCoreLength::Relative(relative_length) => {
                                relative_length * span_line_lengths[i_span]
                            },
                            ViscousCoreLength::Absolute(length) => length,
                            ViscousCoreLength::NoViscousCore => MIN_POSITIVE
                        };

                        let length_before_last_value_is_used = (wake_indices.nr_panels_per_line_element - 1) as Float * evolution_settings.evolution_length_factor;

                        let relative_stream_value = (i_stream as Float / length_before_last_value_is_used).min(1.0).max(0.0); // Converts to [0, 1]

                        let sin_input = 0.5 * relative_stream_value * PI; // Converts to [0, PI/2]
                        let sin_factor = sin_input.sin(); // Converts to [0, 1]

                        viscous_core_length_raw * (1.0 - sin_factor) +
                        final_viscous_core_length * sin_factor
                    }
                };
            }
        }

        out
    }

    pub fn get_wake_indices(
        &self,
        line_force_model: &LineForceModel,
    ) -> WakeIndices {
        let span_points = line_force_model.span_points();

        let nr_panels_along_span = line_force_model.nr_span_lines();
        let nr_points_along_span = span_points.len();

        let nr_panels_per_line_element = self.nr_panels_per_line_element;
        let nr_points_per_line_element = self.nr_panels_per_line_element + 1;

        WakeIndices {
            nr_points_along_span,
            nr_panels_along_span,
            nr_panels_per_line_element,
            nr_points_per_line_element,
        }
    }

}

impl Default for WakeBuilder {
    fn default() -> Self {
        Self {
            nr_panels_per_line_element: Self::default_number_of_panels_per_line_element(),
            viscous_core_length: Default::default(),
            viscous_core_length_evolution: Default::default(),
            first_panel_relative_length: Self::default_first_panel_relative_length(),
            last_panel_relative_length: Self::default_last_panel_relative_length(),
            use_chord_direction: false,
            symmetry_condition: Default::default(),
            ratio_of_wake_affected_by_induced_velocities: Default::default(),
            far_field_ratio: PotentialTheorySettings::default_far_field_ratio(),
            shape_damping_factor: 0.0,
            neglect_self_induced_velocities: false,
            initial_relative_wake_length: Self::default_initial_relative_wake_length(),
            write_wake_data_to_file: false,
            wake_files_folder_path: String::new(),
        }
    }
}

impl SteadyWakeBuilder {
    pub fn default_wake_length_factor() -> Float {100.0}

    pub fn build(&self,
        line_force_model: &LineForceModel,
    ) -> Wake {
        WakeBuilder {
            nr_panels_per_line_element: 1,
            viscous_core_length: self.viscous_core_length.clone(),
            viscous_core_length_evolution: Default::default(),
            first_panel_relative_length: WakeBuilder::default_first_panel_relative_length(),
            last_panel_relative_length: self.wake_length_factor,
            use_chord_direction: false,
            symmetry_condition: self.symmetry_condition.clone(),
            ratio_of_wake_affected_by_induced_velocities: 0.0,
            far_field_ratio: INFINITY,
            shape_damping_factor: 0.0,
            neglect_self_induced_velocities: false,
            initial_relative_wake_length: WakeBuilder::default_initial_relative_wake_length(),
            write_wake_data_to_file: false,
            wake_files_folder_path: String::new(),
        }.build(line_force_model)
    }
}

impl Default for SteadyWakeBuilder {
    fn default() -> Self {
        Self {
            wake_length_factor: Self::default_wake_length_factor(),
            symmetry_condition: Default::default(),
            viscous_core_length: Default::default(),
        }
    }
}
