// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Structures and logic for building a [line force model](crate::line_force_model::LineForceModel).

use serde::{Serialize, Deserialize};
use serde_json;

pub mod single_wing;

use super::*;

use single_wing::WingBuilder;

use crate::error::Error;

use corrections::circulation::CirculationCorrectionBuilder;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineForceModelBuilder {
    pub wing_builders: Vec<WingBuilder>,
    /// Nr sections to discretize the wing into. That is, each wing in the wing builder vector will
    /// end up having a number of sections equal to this variable.
    pub nr_sections: usize,
    #[serde(default = "LineForceModel::default_density")]
    pub density: f64,
    #[serde(default)]
    pub circulation_correction: CirculationCorrectionBuilder,
    #[serde(default)]
    pub angle_of_attack_correction: AngleOfAttackCorrection,
    #[serde(default)]
    pub output_coordinate_system: CoordinateSystem,
    #[serde(default)]
    pub rotation_type: RotationType,
    #[serde(default)]
    pub local_wing_angles: Vec<f64>,
    #[serde(default)]
    pub rotation: SpatialVector<3>,
    #[serde(default)]
    pub translation: SpatialVector<3>,
}

impl LineForceModelBuilder {
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
            rotation: SpatialVector([0.0, 0.0, 0.0]),
            translation: SpatialVector([0.0, 0.0, 0.0]),
        }
    }

    pub fn new_from_string(setup_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(setup_string)?;

        Ok(serde_res)
    }

    pub fn add_wing(&mut self, wing_builder: WingBuilder) {
        self.wing_builders.push(wing_builder);
    }

    pub fn build(&self) -> LineForceModel {
        self.build_with_nr_sections(self.nr_sections)
    } 

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

        if self.local_wing_angles.len() > 0 {
            if self.local_wing_angles.len() != line_force_model.nr_wings() {
                panic!("The number of local wing angles does not match the number of wings.");
            }

            line_force_model.local_wing_angles = self.local_wing_angles.clone();
        }
        

        line_force_model.rigid_body_motion.translation = self.translation;
        line_force_model.rigid_body_motion.rotation = self.rotation;

        line_force_model
    }    
}



