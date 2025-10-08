
use serde::{Serialize, Deserialize};

use stormath::{
    spatial_vector::SpatialVector,
    interpolation::linear_interpolation,
    type_aliases::Float
};

use crate::section_models::SectionModel;
use crate::line_force_model::span_line::SpanLine;

/// Input struct to add a single wing to a line force model
pub struct SingleWing {
    pub span_lines_local: Vec<SpanLine>,
    pub chord_vectors_local: Vec<SpatialVector>,
    pub chord_lengths: Vec<Float>,
    pub section_model: SectionModel,
    pub non_zero_circulation_at_ends: [bool; 2],
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A wing is specified by giving a set of points along the span were the chord length and 
/// section model is set. Between these points the chord length and section model is linearly 
/// interpolated 
pub struct WingBuilder {
    pub section_points: Vec<SpatialVector>,
    pub chord_vectors: Vec<SpatialVector>,
    pub section_model: SectionModel,
    pub non_zero_circulation_at_ends: [bool; 2],
    #[serde(default)]
    pub nr_sections: Option<usize>,
}

impl WingBuilder {
    pub fn span_distance(&self) -> Vec<Float> {
        let mut span_distance: Vec<Float> = Vec::new();

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

    pub fn build(&self, default_nr_sections: usize) -> SingleWing {
        // TODO: add functionality to handle varying foil models across the span!

        let nr_sections = self.nr_sections.unwrap_or(default_nr_sections);

        let span_distance = self.span_distance();

        let total_span_distance = span_distance.last().unwrap();

        let delta_span_distance = total_span_distance / (nr_sections as Float);        

        let mut span_lines_local: Vec<SpanLine> = Vec::new();
        let mut chord_vectors_local: Vec<SpatialVector> = Vec::new();
        let mut chord_lengths: Vec<Float> = Vec::new();

        for i in 0..nr_sections {
            let start_distance = i as Float * delta_span_distance;
            let end_distance   = (i+1) as Float * delta_span_distance;
            let ctrl_point_distance = 0.5 * (start_distance + end_distance);

            let start_point = linear_interpolation(start_distance, &span_distance, &self.section_points);
            let end_point   = linear_interpolation(end_distance,   &span_distance, &self.section_points);

            span_lines_local.push(SpanLine{start_point, end_point});

            chord_vectors_local.push(
                linear_interpolation(ctrl_point_distance, &span_distance, &self.chord_vectors)
            );

            chord_lengths.push(chord_vectors_local.last().unwrap().length());
        }

        let section_model = match &self.section_model {
            SectionModel::Foil(foil) => SectionModel::Foil(foil.clone()),
            SectionModel::VaryingFoil(foils) => SectionModel::VaryingFoil(foils.clone()),
            SectionModel::RotatingCylinder(cylinder) => SectionModel::RotatingCylinder(cylinder.clone()),
            SectionModel::EffectiveWindSensor => SectionModel::EffectiveWindSensor
        };

        SingleWing {
            span_lines_local,
            chord_vectors_local,
            chord_lengths,
            section_model,
            non_zero_circulation_at_ends: self.non_zero_circulation_at_ends,
        }
    }
}