// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};
use serde_json;

use super::*;

use super::single_wing::WingBuilder;

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
    pub smoothing_settings: Option<SmoothingSettings>,
    #[serde(default)]
    pub ctrl_point_chord_factor: f64,
}

impl LineForceModelBuilder {
    pub fn new(nr_sections: usize) -> Self {
        LineForceModelBuilder {
            wing_builders: Vec::new(),
            nr_sections,
            density: LineForceModel::default_density(),
            smoothing_settings: None,
            ctrl_point_chord_factor: 0.0,
        }
    }

    pub fn new_from_string(setup_string: &str) -> Self {
        serde_json::from_str(setup_string).unwrap()
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

        line_force_model.smoothing_settings = self.smoothing_settings.clone();
        line_force_model.ctrl_point_chord_factor = self.ctrl_point_chord_factor;

        line_force_model
    }    
}



