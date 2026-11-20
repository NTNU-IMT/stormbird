// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Builders for the lifting line correction and its wake models.

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use crate::lifting_line::singularity_elements::symmetry_condition::SymmetryCondition;
use crate::line_force_model::LineForceModel;
use crate::error::Error;

use super::{
    LiftingLineCorrection,
    CorrectionSettings,
    CorrectionWake,
    straight::StraightWake,
    straight_panels::StraightPanelWake,
};

/// A builder for the [LiftingLineCorrection] struct. See the documentation for
/// [LiftingLineCorrection] for more information on how it works.
///
/// The settings that are common for all wake models are given directly in this struct, while the
/// settings that are specific to a wake model are given in the [CorrectionWakeBuilder].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiftingLineCorrectionBuilder {
    /// The wake model used to compute the correction
    #[serde(default)]
    pub wake: CorrectionWakeBuilder,
    /// Optional symmetry condition
    #[serde(default)]
    pub symmetry_condition: SymmetryCondition,
    /// Optional time over which the correction is gradually increased from zero, to avoid large
    /// corrections at the start of a simulation
    #[serde(default)]
    pub initialization_time: Option<Float>,
    /// The viscous core length of the projection width divided by this factor gives the small
    /// reference viscous core length
    #[serde(default="LiftingLineCorrectionBuilder::default_reference_core_length_factor")]
    pub reference_core_length_factor: Float,
    /// If true, the induced velocities from all wings are included in the correction
    #[serde(default)]
    pub include_wing_interactions: bool,
    /// Optional limit on the magnitude of the velocity correction at each control point, given as
    /// a ratio of the magnitude of the sampled velocity at the same point. No limit by default.
    #[serde(default)]
    pub max_correction_ratio: Option<Float>,
}

impl Default for LiftingLineCorrectionBuilder {
    fn default() -> Self {
        Self {
            wake: CorrectionWakeBuilder::default(),
            symmetry_condition: SymmetryCondition::NoSymmetry,
            initialization_time: None,
            reference_core_length_factor: Self::default_reference_core_length_factor(),
            include_wing_interactions: false,
            max_correction_ratio: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// The wake model used in the lifting line correction, with the settings that are specific to each
/// model.
pub enum CorrectionWakeBuilder {
    /// One straight horseshoe vortex for each line element
    Straight(StraightWakeBuilder),
    /// A straight wake, split into panels in the streamwise direction, so that the reference core
    /// length can increase downstream
    StraightPanels(StraightPanelWakeBuilder),
}

impl Default for CorrectionWakeBuilder {
    fn default() -> Self {
        Self::Straight(StraightWakeBuilder::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for a straight wake, with one horseshoe vortex for each line element.
pub struct StraightWakeBuilder {
    /// The length of the wake, relative to the average chord length
    #[serde(default="default_wake_length_factor")]
    pub wake_length_factor: Float,
    /// Length of a virtual extension of each wing, at the first and the last end, relative to the
    /// chord length at that end. The extensions are only used in the reference model with the small
    /// viscous core, and carry the circulation of the outermost line element. They move the tip
    /// vortices away from the ends of the wing, which can be used to model end plates. Zero by
    /// default, which means no extensions.
    #[serde(default)]
    pub virtual_extension_factors: [Float; 2],
}

impl Default for StraightWakeBuilder {
    fn default() -> Self {
        Self {
            wake_length_factor: default_wake_length_factor(),
            virtual_extension_factors: [0.0, 0.0],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for a straight wake, split into panels in the streamwise direction.
pub struct StraightPanelWakeBuilder {
    /// The length of the wake, relative to the average chord length
    #[serde(default="default_wake_length_factor")]
    pub wake_length_factor: Float,
    /// Number of panels in the streamwise direction behind each line element
    #[serde(default="StraightPanelWakeBuilder::default_nr_panels_per_line_element")]
    pub nr_panels_per_line_element: usize,
    /// Length of the first panel, relative to the average chord length. The panels get gradually
    /// longer downstream, so that the total length is given by the `wake_length_factor`.
    #[serde(default="StraightPanelWakeBuilder::default_first_panel_relative_length")]
    pub first_panel_relative_length: Float,
    /// Optional distance along the wake, relative to the average chord length, over which the
    /// reference core length grows from the small reference value to the viscous core length of
    /// the projection width. The growth follows a quarter of a sine period. The two wake models are
    /// then equal downstream, so that the far wake does not contribute to the correction. The
    /// reference core length is constant if not given.
    #[serde(default)]
    pub reference_core_evolution_length_factor: Option<Float>,
    /// Determines how far away from a panel it is necessary to be before the far field method is
    /// used to calculate the induced velocity, rather than the full method.
    #[serde(default="default_far_field_ratio")]
    pub far_field_ratio: Float,
}

impl StraightPanelWakeBuilder {
    fn default_nr_panels_per_line_element() -> usize {50}
    fn default_first_panel_relative_length() -> Float {0.5}
}

impl Default for StraightPanelWakeBuilder {
    fn default() -> Self {
        Self {
            wake_length_factor: default_wake_length_factor(),
            nr_panels_per_line_element: Self::default_nr_panels_per_line_element(),
            first_panel_relative_length: Self::default_first_panel_relative_length(),
            reference_core_evolution_length_factor: None,
            far_field_ratio: default_far_field_ratio(),
        }
    }
}

fn default_wake_length_factor() -> Float {100.0}
fn default_far_field_ratio() -> Float {5.0}

/// Collects error messages for a set of checks
fn check(error_messages: &mut Vec<String>, is_valid: bool, message: String) {
    if !is_valid {
        error_messages.push(message);
    }
}

/// Checks that an optional reference core evolution length is positive
fn check_reference_core_evolution_length(error_messages: &mut Vec<String>, value: Option<Float>) {
    if let Some(value) = value {
        check(
            error_messages,
            value > 0.0,
            format!("reference_core_evolution_length_factor must be positive, but is {}.", value)
        );
    }
}

impl StraightWakeBuilder {
    fn check_settings(&self, error_messages: &mut Vec<String>) {
        check(
            error_messages,
            self.wake_length_factor > 0.0,
            format!("wake_length_factor must be positive, but is {}.", self.wake_length_factor)
        );

        check(
            error_messages,
            self.virtual_extension_factors.iter().all(|factor| *factor >= 0.0),
            format!(
                "virtual_extension_factors can not be negative, but are {:?}.",
                self.virtual_extension_factors
            )
        );
    }
}

impl StraightPanelWakeBuilder {
    fn check_settings(&self, error_messages: &mut Vec<String>) {
        check(
            error_messages,
            self.nr_panels_per_line_element >= 2,
            format!(
                "nr_panels_per_line_element must be at least 2, but is {}. Use the Straight wake \
                for single-panel wakes.",
                self.nr_panels_per_line_element
            )
        );

        check(
            error_messages,
            self.first_panel_relative_length > 0.0 &&
                self.first_panel_relative_length < self.wake_length_factor,
            format!(
                "first_panel_relative_length must be positive and smaller than the \
                wake_length_factor ({}), but is {}.",
                self.wake_length_factor,
                self.first_panel_relative_length
            )
        );

        check(
            error_messages,
            self.far_field_ratio > 0.0,
            format!("far_field_ratio must be positive, but is {}.", self.far_field_ratio)
        );

        check_reference_core_evolution_length(
            error_messages,
            self.reference_core_evolution_length_factor
        );
    }
}

impl LiftingLineCorrectionBuilder {
    fn default_reference_core_length_factor() -> Float {100.0}

    /// Checks that the settings are valid. All checks are executed, so that the returned error
    /// lists every problem with the settings, not just the first.
    pub fn check_settings(&self) -> Result<(), Error> {
        let mut error_messages: Vec<String> = Vec::new();

        check(
            &mut error_messages,
            self.reference_core_length_factor > 0.0,
            format!(
                "reference_core_length_factor must be positive, but is {}.",
                self.reference_core_length_factor
            )
        );

        if let Some(max_correction_ratio) = self.max_correction_ratio {
            check(
                &mut error_messages,
                max_correction_ratio >= 0.0,
                format!("max_correction_ratio can not be negative, but is {}.", max_correction_ratio)
            );
        }

        match &self.wake {
            CorrectionWakeBuilder::Straight(builder) => builder.check_settings(&mut error_messages),
            CorrectionWakeBuilder::StraightPanels(builder) => builder.check_settings(&mut error_messages),
        }

        if error_messages.is_empty() {
            Ok(())
        } else {
            Err(Error::InvalidInput(format!(
                "Invalid lifting line correction settings: {}",
                error_messages.join(" ")
            )))
        }
    }

    /// Builds the correction, where the viscous core length is the given factor times the average
    /// chord length
    ///
    /// # Errors
    /// If the settings are not valid, see [`check_settings`](Self::check_settings).
    pub fn build(
        &self,
        viscous_core_length_factor: Float,
        line_force_model: &LineForceModel
    ) -> Result<LiftingLineCorrection, Error> {
        self.check_settings()?;

        let viscous_core_length = viscous_core_length_factor * line_force_model.average_chord_length();

        let settings = CorrectionSettings {
            viscous_core_length,
            reference_core_length: viscous_core_length / self.reference_core_length_factor,
            symmetry_condition: self.symmetry_condition,
            include_wing_interactions: self.include_wing_interactions,
        };

        let wake = match &self.wake {
            CorrectionWakeBuilder::Straight(builder) => {
                CorrectionWake::Straight(StraightWake {
                    wake_length_factor: builder.wake_length_factor,
                    virtual_extension_factors: builder.virtual_extension_factors,
                })
            },
            CorrectionWakeBuilder::StraightPanels(builder) => {
                CorrectionWake::StraightPanels(
                    StraightPanelWake::new(builder, &settings, line_force_model)?
                )
            },
        };

        Ok(LiftingLineCorrection {
            settings,
            initialization_time: self.initialization_time,
            max_correction_ratio: self.max_correction_ratio,
            wake,
        })
    }
}
