// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Functionality for calculating lift-induced velocities from full dynamic wake.

use serde::{Serialize, Deserialize};

use math_utils::spatial_vector::SpatialVector;

use crate::line_force_model::LineForceModel;

use crate::lifting_line::singularity_elements::prelude::*;

use super::{
    Wake,
    settings::*
};

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
        Self::NrPanels(100)
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Enum to represent different ways a viscous core length can be specified.
pub enum ViscousCoreLength {
    /// Signifies that the viscous core length is a fraction of the length of the vortex line. To
    /// be used, the vortex line geometry must be known.
    Relative(f64),
    /// Signifies that the viscous core length is an absolute value, and that it can be used without
    /// any more information about the geometry.
    Absolute(f64),
    /// Signifies that the viscous core length is not used.
    NoViscousCore,
}

impl Default for ViscousCoreLength {
    fn default() -> Self {
        Self::Relative(0.1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Variables used to build a wake model.
pub struct WakeBuilder {
    #[serde(default)]
    /// Data used to determine the length of the wake.
    pub wake_length: WakeLength,
    #[serde(default)]
    /// The viscous core length used when calculating the induced velocities
    pub viscous_core_length: ViscousCoreLength,
    #[serde(default)]
    /// The viscous core length at the end of the wake
    pub viscous_core_length_separated: Option<ViscousCoreLength>,
    #[serde(default="WakeBuilder::default_first_panel_relative_length")]
    /// How the first panel in the wake is treated
    pub first_panel_relative_length: f64,
    #[serde(default="WakeBuilder::default_last_panel_relative_length")]
    /// Factor used to calculate the length of the final panel, relative to the chord length.
    pub last_panel_relative_length: f64,
    #[serde(default)]
    /// Determines if the chord direction should be used when calculating the direction of the first
    /// wake panels
    pub use_chord_direction: bool,
    #[serde(default)]
    /// Determines the damping factor for the wake strength. Specifies how much damping there should
    /// be on the last panel. The actual damping factor also depends on the number of wake panels.
    pub strength_damping: StrengthDamping,
    #[serde(default)]
    /// Symmetry condition
    pub symmetry_condition: SymmetryCondition,
    #[serde(default)]
    /// Ratio that determines how much of the wake points that are affected by the induced
    /// velocities. The default is zero, which means no wake points are affected by the induced
    /// velocities. A value of 1.0 means that all wake points are affected by the induced
    /// velocities.
    pub ratio_of_wake_affected_by_induced_velocities: f64,
    #[serde(default="PotentialTheorySettings::default_far_field_ratio")]
    /// Determines how far away from a panel it is necessary to be before the far field method is
    /// used to calculate the induced velocity, rather than the full method.
    pub far_field_ratio: f64,
    #[serde(default)]
    /// Damping factor for the shape of the wake. A value of 0.0 means no damping (the wake moves
    /// freely), while a value of 1.0 means that the wake points are fixed in space.
    pub shape_damping_factor: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Variables used to build a steady wake model.
pub struct SteadyWakeBuilder {
    #[serde(default="SteadyWakeBuilder::default_wake_length_factor")]
    pub wake_length_factor: f64,
    #[serde(default)]
    pub symmetry_condition: SymmetryCondition,
    #[serde(default)]
    pub viscous_core_length: ViscousCoreLength,
}


impl WakeBuilder {
    fn default_first_panel_relative_length() -> f64 {0.75}
    fn default_last_panel_relative_length() -> f64 {25.0}

    pub fn build(
        &self,
        time_step: f64,
        line_force_model: &LineForceModel,
        initial_velocity: SpatialVector<3>,
    ) -> Wake {
        let span_points = line_force_model.span_points();

        let indices = self.get_wake_indices(time_step, line_force_model, initial_velocity);

        let wake_building_velocity = if initial_velocity.length() == 0.0 {
            SpatialVector::<3>::new(1e-6, 1e-6, 1e-6)
        } else {
            initial_velocity
        };

        let mut points = vec![SpatialVector::<3>::default(); indices.nr_points()];

        for i_stream in 0..indices.nr_points_per_line_element {
            for i_span in 0..span_points.len() {
                let flat_index = indices.point_index(i_stream, i_span);

                points[flat_index] = span_points[i_span];
            }
        }

        let end_index_induced_velocities_on_wake = (
            self.ratio_of_wake_affected_by_induced_velocities *
            indices.nr_panels_per_line_element as f64
        ).ceil() as usize;

        let viscous_core_length = self.get_viscous_core_length(line_force_model);

        let settings = WakeSettings {
            first_panel_relative_length: self.first_panel_relative_length,
            last_panel_relative_length: self.last_panel_relative_length,
            use_chord_direction: self.use_chord_direction,
            strength_damping: self.strength_damping,
            viscous_core_length,
            end_index_induced_velocities_on_wake,
            shape_damping_factor: self.shape_damping_factor,
            neglect_self_induced_velocities: self.neglect_self_induced_velocities
        };

        let potential_theory_settings = PotentialTheorySettings {
            symmetry_condition: self.symmetry_condition.clone(),
            far_field_ratio: self.far_field_ratio,
            ..Default::default()
        };

        let nr_panels = indices.nr_panels();

        let undamped_strengths: Vec<f64> = vec![0.0; nr_panels];
        let strengths: Vec<f64> = vec![0.0; nr_panels];
        let panels_lifetime: Vec<f64> = vec![0.0; nr_panels];
        let panels_strength_damping_factor: Vec<f64> = vec![0.0; nr_panels];

        let panels_viscous_core_length = vec![viscous_core_length.value; nr_panels];

        let panels = vec![Panel::default(); nr_panels];

        let mut wake = Wake {
            indices,
            points,
            undamped_strengths,
            strengths,
            panels_lifetime,
            panels_strength_damping_factor,
            panels_viscous_core_length,
            settings,
            potential_theory_settings,
            wing_indices: line_force_model.wing_indices.clone(),
            number_of_time_steps_completed: 0,
            panels
        };

        wake.initialize(line_force_model, wake_building_velocity, time_step);

        wake
    }

    pub fn get_viscous_core_length(&self, line_force_model: &LineForceModel) -> SeparationDependentValue {
        let span_lines = line_force_model.span_lines();

        let average_line_element_length: f64 = span_lines.iter()
            .map(|line| line.length())
            .sum::<f64>() / span_lines.len() as f64;

        let value = match self.viscous_core_length {
            ViscousCoreLength::Relative(relative_length) => {
                relative_length * average_line_element_length
            },
            ViscousCoreLength::Absolute(length) => length,
            ViscousCoreLength::NoViscousCore => f64::MIN
        };

        let value_separated = match self.viscous_core_length_separated {
            Some(ViscousCoreLength::Relative(relative_length)) => {
                Some(relative_length * average_line_element_length)
            },
            Some(ViscousCoreLength::Absolute(length)) => Some(length),
            Some(ViscousCoreLength::NoViscousCore) => Some(f64::MIN),
            None => None
        };

        SeparationDependentValue {
            value,
            value_separated
        }
    }

    pub fn get_wake_indices(
        &self,
        time_step: f64,
        line_force_model: &LineForceModel,
        initial_velocity: SpatialVector<3>
    ) -> WakeIndices {
        let span_points   = line_force_model.span_points();
        let chord_vectors = line_force_model.global_chord_vectors();

        let nr_panels_along_span = line_force_model.nr_span_lines();
        let nr_points_along_span = span_points.len();

        let mean_chord_length: f64 = chord_vectors.iter()
            .map(|chord| chord.length())
            .sum::<f64>() / chord_vectors.len() as f64;

        let nr_panels_per_line_element = match self.wake_length {
            WakeLength::NrPanels(nr_panels) => nr_panels,
            WakeLength::TargetLengthFactor(_) => {
                if initial_velocity.length() <= 1e-7 {
                    panic!("Freestream velocity is zero. Cannot calculate wake length.");
                }

                self.wake_length.nr_wake_panels_from_target_length_factor(
                    mean_chord_length, initial_velocity.length(), time_step
                ).unwrap()
            }
        };

        let nr_points_per_line_element = nr_panels_per_line_element + 1;

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
            wake_length: Default::default(),
            viscous_core_length: Default::default(),
            viscous_core_length_separated: None,
            first_panel_relative_length: Self::default_first_panel_relative_length(),
            last_panel_relative_length: Self::default_last_panel_relative_length(),
            use_chord_direction: false,
            strength_damping: Default::default(),
            symmetry_condition: Default::default(),
            ratio_of_wake_affected_by_induced_velocities: Default::default(),
            far_field_ratio: PotentialTheorySettings::default_far_field_ratio(),
            shape_damping_factor: 0.0,
            neglect_self_induced_velocities: false
        }
    }
}

impl SteadyWakeBuilder {
    pub fn default_wake_length_factor() -> f64 {100.0}

    pub fn build(&self,
        time_step: f64,
        line_force_model: &LineForceModel,
        initial_velocity: SpatialVector<3>
    ) -> Wake {
        WakeBuilder {
            wake_length: WakeLength::NrPanels(1),
            viscous_core_length: self.viscous_core_length.clone(),
            viscous_core_length_separated: None,
            first_panel_relative_length: WakeBuilder::default_first_panel_relative_length(),
            last_panel_relative_length: self.wake_length_factor,
            use_chord_direction: false,
            strength_damping: Default::default(),
            symmetry_condition: self.symmetry_condition.clone(),
            ratio_of_wake_affected_by_induced_velocities: 0.0,
            far_field_ratio: f64::INFINITY,
            shape_damping_factor: 0.0,
            neglect_self_induced_velocities: false
        }.build(time_step, line_force_model, initial_velocity)
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
