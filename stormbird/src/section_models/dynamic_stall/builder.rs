// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Builder for the [dynamic stall model](DynamicStallModel).

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

use super::{DynamicStallModel, DynamicStallSegmentState};
use crate::section_models::SectionModel;
use crate::error::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for a [dynamic stall model](DynamicStallModel) on a single wing. Only contains the
/// parameters of the model. The internal state of the model, i.e., the memory effects for each line
/// segment, is created when the model is built, as the number of line segments is then known.
///
/// The parameters for the stall in the foil model are optional, so that it is possible to check
/// that they are not set for section models that are not built on the foil model. Values that are
/// not set use the default values when the model is built.
pub struct DynamicStallModelBuilder {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Non-dimensional time constant for the lag in the pressure distribution, T_p, given in
    /// semi-chords travelled. A value of zero turns the lag off. Only used for section models that
    /// are built on the foil model.
    pub pressure_time_constant: Option<Float>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Non-dimensional time constant for the lag in the boundary layer separation, T_f, given in
    /// semi-chords travelled. A value of zero turns the lag off. Only used for section models that
    /// are built on the foil model.
    pub separation_time_constant: Option<Float>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Non-dimensional time constant for the decay of the leading edge vortex lift, T_v, given in
    /// semi-chords travelled. A value of zero turns the leading edge vortex off. Only used for
    /// section models that are built on the foil model.
    pub vortex_time_constant: Option<Float>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Non-dimensional time it takes for the leading edge vortex to pass the trailing edge, T_vl,
    /// given in semi-chords travelled. The vortex is only fed while it is above the foil. Only used
    /// for section models that are built on the foil model.
    pub vortex_passage_time: Option<Float>,
    #[serde(default)]
    /// **Experimental**. Non-dimensional time constant for the lag in the internal state of the
    /// section model, T_i, given in semi-chords travelled. Used for all section models. For a
    /// rotating cylinder, the internal state is the spin ratio. The default value is zero, which
    /// turns the lag off.
    pub internal_state_time_constant: Float,
}

impl DynamicStallModelBuilder {
    /// Default value for the pressure time constant, typical for a NACA 0012 at low Mach numbers
    pub fn default_pressure_time_constant() -> Float {1.7}
    /// Default value for the separation time constant, typical for a NACA 0012 at low Mach numbers
    pub fn default_separation_time_constant() -> Float {3.0}
    /// Default value for the vortex time constant, typical for a NACA 0012 at low Mach numbers
    pub fn default_vortex_time_constant() -> Float {6.0}
    /// Default value for the vortex passage time, typical for a NACA 0012 at low Mach numbers
    pub fn default_vortex_passage_time() -> Float {7.0}

    /// Returns true if any of the parameters for the stall in the foil model are set
    pub fn has_foil_parameters(&self) -> bool {
        self.pressure_time_constant.is_some() ||
        self.separation_time_constant.is_some() ||
        self.vortex_time_constant.is_some() ||
        self.vortex_passage_time.is_some()
    }

    /// Checks that the builder is valid for the given section model, i.e., that the time
    /// constants are valid, see
    /// [`check_time_constants`](DynamicStallModelBuilder::check_time_constants), and that the
    /// parameters for the stall in the foil model are only set for section models that are built
    /// on the foil model.
    pub fn check(&self, section_model: &SectionModel) -> Result<(), Error> {
        self.check_time_constants()?;

        if self.has_foil_parameters() && !DynamicStallModel::uses_foil_stall_model(section_model) {
            return Err(Error::CustomStringError(
                "The parameters for the stall in the dynamic stall model, i.e., \
                pressure_time_constant, separation_time_constant, vortex_time_constant and \
                vortex_passage_time, are only used together with section models that build on the \
                foil model, i.e., single element foils, multi element foils and active air flow \
                foils. Only the internal_state_time_constant can be set for other section models."
                    .to_string()
            ));
        }

        Ok(())
    }

    /// Checks that none of the time constants are negative. Values that are not set use the
    /// default values, which are always valid.
    pub fn check_time_constants(&self) -> Result<(), Error> {
        let time_constants = [
            ("pressure_time_constant", self.pressure_time_constant),
            ("separation_time_constant", self.separation_time_constant),
            ("vortex_time_constant", self.vortex_time_constant),
            ("vortex_passage_time", self.vortex_passage_time),
            ("internal_state_time_constant", Some(self.internal_state_time_constant)),
        ];

        for (name, value) in time_constants {
            if let Some(value) = value {
                if value < 0.0 {
                    return Err(Error::CustomStringError(format!(
                        "The time constants in the dynamic stall model can not be negative, but \
                        {} = {}", name, value
                    )));
                }
            }
        }

        Ok(())
    }

    /// Builds the dynamic stall model for a wing with the given number of line segments.
    ///
    /// # Panics
    /// If any of the time constants are negative.
    pub fn build(&self, nr_segments: usize) -> DynamicStallModel {
        let pressure_time_constant = self.pressure_time_constant
            .unwrap_or_else(Self::default_pressure_time_constant);
        let separation_time_constant = self.separation_time_constant
            .unwrap_or_else(Self::default_separation_time_constant);
        let vortex_time_constant = self.vortex_time_constant
            .unwrap_or_else(Self::default_vortex_time_constant);
        let vortex_passage_time = self.vortex_passage_time
            .unwrap_or_else(Self::default_vortex_passage_time);

        if let Err(error) = self.check_time_constants() {
            panic!("{}", error);
        }

        DynamicStallModel {
            pressure_time_constant,
            separation_time_constant,
            vortex_time_constant,
            vortex_passage_time,
            internal_state_time_constant: self.internal_state_time_constant,
            time_step: 0.0,
            segment_states: vec![DynamicStallSegmentState::default(); nr_segments],
            is_initialized: false,
        }
    }
}
