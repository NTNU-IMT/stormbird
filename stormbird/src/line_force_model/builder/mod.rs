// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Structures and logic for building a [line force model](crate::line_force_model::LineForceModel).

use serde::{Serialize, Deserialize};
use serde_json;

/// Builder for a single wing
pub mod single_wing;

use super::*;

use single_wing::WingBuilder;

use crate::error::Error;
use super::force_calculations::ForceCalculationSettings;

use corrections::circulation::CirculationCorrectionBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for a line force model, consisting of one or more wings
pub struct LineForceModelBuilder {
    /// Builders for each wing in the model
    pub wing_builders: Vec<WingBuilder>,
    /// Nr sections to discretize the wing into. That is, each wing in the wing builder vector will
    /// end up having a number of sections equal to this variable.
    pub nr_sections: usize,
    /// The density of the fluid
    #[serde(default = "LineForceModel::default_density")]
    pub density: Float,
    /// Optional correction of the circulation distribution
    #[serde(default)]
    pub circulation_correction: CirculationCorrectionBuilder,
    /// Optional correction of the angle of attack
    #[serde(default)]
    pub angle_of_attack_correction: AngleOfAttackCorrection,
    /// The coordinate system for the output forces
    #[serde(default)]
    pub output_coordinate_system: CoordinateSystem,
    /// How the rotation angles of the rigid body motion are applied
    #[serde(default)]
    pub rotation_type: RotationType,
    /// Initial local angles for each wing, i.e., rotation around the span axis of each wing
    #[serde(default)]
    pub local_wing_angles: Vec<Float>,
    /// Initial rotation of the whole model
    #[serde(default)]
    pub rotation: SpatialVector,
    /// Initial translation of the whole model
    #[serde(default)]
    pub translation: SpatialVector,
    /// Settings for the force calculations
    #[serde(default)]
    pub force_calculation_settings: ForceCalculationSettings
}

impl LineForceModelBuilder {
    /// Creates a new builder without any wings, where each wing is discretized into the given
    /// number of sections
    pub fn new(nr_sections: usize) -> Self {
        LineForceModelBuilder {
            wing_builders: Vec::new(),
            nr_sections,
            density: LineForceModel::default_density(),
            circulation_correction: Default::default(),
            angle_of_attack_correction: Default::default(),
            output_coordinate_system: CoordinateSystem::Global,
            rotation_type: RotationType::XYZ,
            local_wing_angles: Vec::new(),
            rotation: SpatialVector::from([0.0, 0.0, 0.0]),
            translation: SpatialVector::from([0.0, 0.0, 0.0]),
            force_calculation_settings: ForceCalculationSettings::default()
        }
    }

    /// Creates a builder from a JSON string
    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(setup_string)?;

        Ok(serde_res)
    }

    /// Adds a wing to the model
    pub fn add_wing(&mut self, wing_builder: WingBuilder) {
        self.wing_builders.push(wing_builder);
    }

    /// Checks that the settings for each wing are consistent. At the moment, this checks that the
    /// dynamic stall models, if any, are valid for the section models, see
    /// [`WingBuilder::check_dynamic_stall`].
    pub fn check_settings(&self) -> Result<(), Error> {
        for wing_builder in &self.wing_builders {
            wing_builder.check_dynamic_stall()?;
        }

        Ok(())
    }

    /// Builds the line force model
    pub fn build(&self) -> LineForceModel {
        self.build_with_nr_sections(self.nr_sections)
    } 

    /// Builds the line force model with the given number of sections for each wing, rather than the
    /// number in the builder
    pub fn build_with_nr_sections(&self, nr_sections: usize) -> LineForceModel {
        let mut line_force_model = LineForceModel::new(self.density);

        for wing_builder in &self.wing_builders {
            let wing = wing_builder.build(nr_sections);

            line_force_model.add_wing(&wing);
        }

        let circulation_correction = self.circulation_correction.build(
            &line_force_model
        );

        line_force_model.circulation_correction = circulation_correction;
        line_force_model.angle_of_attack_correction = self.angle_of_attack_correction.clone();
        
        line_force_model.output_coordinate_system = self.output_coordinate_system;

        if !self.local_wing_angles.is_empty() {
            if self.local_wing_angles.len() != line_force_model.nr_wings() {
                panic!("The number of local wing angles does not match the number of wings.");
            }

            line_force_model.local_wing_angles = self.local_wing_angles.clone();
        }
        
        line_force_model.force_calculation_settings = self.force_calculation_settings.clone();

        line_force_model.rigid_body_motion.translation = self.translation;
        line_force_model.rigid_body_motion.rotation = self.rotation;

        line_force_model.update_calculated_values_after_create();

        line_force_model
    }    
}



