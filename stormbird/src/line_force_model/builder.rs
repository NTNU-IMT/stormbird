// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use serde::{Serialize, Deserialize};
use serde_json;

use crate::math_utils::interpolation::linear_interpolation;

use super::*;

use crate::section_models::SectionModel;

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
    pub prescribed_circulation: Option<PrescribedCirculation>,
}

impl LineForceModelBuilder {
    pub fn new(nr_sections: usize) -> Self {
        LineForceModelBuilder {
            wing_builders: Vec::new(),
            nr_sections,
            density: LineForceModel::default_density(),
            smoothing_settings: None,
            prescribed_circulation: None,
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
        line_force_model.prescribed_circulation = self.prescribed_circulation.clone();

        line_force_model
    }    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A wing is specified by giving a set of points along the span were the chord length and 
/// section model is set. Between these points the chord length and section model is linearly 
/// interpolated 
pub struct WingBuilder {
    pub section_points: Vec<Vec3>,
    pub chord_vectors: Vec<Vec3>,
    pub section_model: SectionModel,
}

impl WingBuilder {
    pub fn span_distance(&self) -> Vec<f64> {
        let mut span_distance: Vec<f64> = Vec::new();

        for i in 0..self.section_points.len() {
            if span_distance.is_empty() {
                span_distance.push(0.0);
            } else {
                let previous_distance = span_distance.last().unwrap();

                let current_point  = self.section_points[i];
                let previous_point = self.section_points[i-1];

                span_distance.push(previous_distance + current_point.distance(previous_point));
            }
        }

        span_distance
    }

    pub fn build(&self, nr_sections: usize) -> SingleWing {
        // TODO: add functionality to handle varying foil models across the span!

        let span_distance = self.span_distance();

        let total_span_distance = span_distance.last().unwrap();

        let delta_span_distance = total_span_distance / (nr_sections as f64);        

        let mut span_lines_local: Vec<SpanLine> = Vec::new();
        let mut chord_vectors_local: Vec<Vec3> = Vec::new();

        for i in 0..nr_sections {
            let start_distance = i as f64 * delta_span_distance;
            let end_distance   = (i+1) as f64 * delta_span_distance;
            let ctrl_point_distance = 0.5 * (start_distance + end_distance);

            let start_point = linear_interpolation(start_distance, &span_distance, &self.section_points);
            let end_point   = linear_interpolation(end_distance,   &span_distance, &self.section_points);

            span_lines_local.push(SpanLine{start_point, end_point});

            chord_vectors_local.push(
                linear_interpolation(ctrl_point_distance, &span_distance, &self.chord_vectors)
            );
        }

        let section_model = match &self.section_model {
            SectionModel::Foil(foil) => SectionModel::Foil(foil.clone()),
            SectionModel::VaryingFoil(foils) => SectionModel::VaryingFoil(foils.clone()),
            SectionModel::RotatingCylinder(cylinder) => SectionModel::RotatingCylinder(cylinder.clone()),
        };

        SingleWing {
            span_lines_local,
            chord_vectors_local,
            section_model,
        }
    }
}

