// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use serde::{Serialize, Deserialize};

use stormath::{
    spatial_vector::SpatialVector,
    interpolation::linear_interpolation,
    type_aliases::Float
};

use crate::section_models::SectionModel;
use crate::line_force_model::span_line::SpanLine;
use crate::line_force_model::input_power::InputPowerModel;
use crate::line_force_model::wing_internal_state::InternalCompartments;
use crate::line_force_model::corrections::force::ForceCorrection;
use crate::line_force_model::added_mass::AddedMass;
use crate::line_force_model::gyroscopic_moment::GyroscopicMoment;

#[derive(Debug, Clone)]
/// Input struct to add a single wing to a line force model
pub struct SingleWing {
    pub span_lines_local: Vec<SpanLine>,
    pub chord_vectors_local: Vec<SpatialVector>,
    pub chord_lengths: Vec<Float>,
    pub line_segment_is_virtual: Vec<bool>,
    pub section_model: SectionModel,
    pub non_zero_circulation_at_ends: [bool; 2],
    pub input_power_model: InputPowerModel,
    pub internal_compartments: InternalCompartments,
    pub force_correction: ForceCorrection,
    pub added_mass: AddedMass,
    pub gyroscopic_moment: GyroscopicMoment,
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
    #[serde(default)]
    pub line_segment_is_virtual: Option<Vec<bool>>,
    #[serde(default="WingBuilder::default_non_zero_circulation_at_ends")]
    pub non_zero_circulation_at_ends: [bool; 2],
    #[serde(default)]
    pub nr_sections: Option<usize>,
    #[serde(default)]
    pub input_power_model: InputPowerModel,
    #[serde(default)]
    /// Optional division of the internal volume of the wing into sealed compartments, given as the
    /// non-dimensional span locations of the boundaries between them, including the ends of the
    /// wing at -0.5 and 0.5. Only used by section models with an internal flow model, such as
    /// suction sails with several fans. The default is a single compartment.
    pub internal_compartment_boundaries: Option<Vec<Float>>,
    #[serde(default)]
    /// Optional empirical corrections of the forces on the wing. Not used by default.
    pub force_correction: ForceCorrection,
    #[serde(default)]
    /// Optional model for the added mass forces on the wing. Not used by default.
    pub added_mass: AddedMass,
    #[serde(default)]
    /// Optional model for the gyroscopic moment from the wing. Only allowed together with the
    /// rotating cylinder section model. Not used by default.
    pub gyroscopic_moment: GyroscopicMoment,
}

impl WingBuilder {
    pub fn default_non_zero_circulation_at_ends() -> [bool; 2] {[false, false]}

    /// Returns a vector that contain the distance from along the span to each section point.
    pub fn specified_sections_span_distance(&self) -> Vec<Float> {
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

    /// Returns the internal compartments of the wing.
    ///
    /// # Panics
    /// If the boundaries are not valid, or if more than one compartment is specified for a section
    /// model that does not use internal compartments.
    pub fn internal_compartments(&self) -> InternalCompartments {
        let Some(boundaries) = &self.internal_compartment_boundaries else {
            return InternalCompartments::default();
        };

        let compartments = InternalCompartments::new(boundaries.clone()).unwrap_or_else(|error| {
            panic!("Invalid internal compartments for a wing: {}", error)
        });

        if compartments.nr_compartments() > 1 && !self.section_model.uses_internal_compartments() {
            panic!(
                "Internal compartments are only used by section models with an internal flow \
                model, i.e., an active air flow foil with the PowerToInternalFlow or \
                FanSpeedToInternalFlow conversion"
            );
        }

        compartments
    }

    /// Returns the gyroscopic moment model of the wing.
    ///
    /// # Panics
    /// If the model is active for a section model that is not a rotating cylinder, as the internal
    /// state is then not the rotational speed of the wing.
    pub fn gyroscopic_moment(&self) -> GyroscopicMoment {
        if self.gyroscopic_moment.is_active() &&
            !matches!(self.section_model, SectionModel::RotatingCylinder(_))
        {
            panic!(
                "The gyroscopic moment model is only used together with the rotating cylinder \
                section model"
            );
        }

        self.gyroscopic_moment.clone()
    }

    /// Builds the data for a single wing
    pub fn build(&self, default_nr_sections: usize) -> SingleWing {
        let nr_specified_sections = self.section_points.len();
        let nr_segments = nr_specified_sections - 1;
        
        let nr_sections_average = self.nr_sections.unwrap_or(default_nr_sections);

        let specified_sections_span_distance = self.specified_sections_span_distance();

        let total_span_distance = specified_sections_span_distance.last().unwrap();

        let ds_average = total_span_distance / (nr_sections_average as Float);

        let mut ds_locally: Vec<Float> = Vec::with_capacity(nr_segments);
        let mut nr_sections_locally: Vec<usize> = Vec::with_capacity(nr_segments);

        for i in 0..nr_segments {
            let segment_distance = specified_sections_span_distance[i+1] - 
                specified_sections_span_distance[i];
            
            nr_sections_locally.push(
                (segment_distance / ds_average).ceil() as usize
            );

            ds_locally.push(
                segment_distance / nr_sections_locally[i] as Float
            );
        }

        let nr_sections_total = nr_sections_locally.iter().sum();

        let mut span_lines_local: Vec<SpanLine> = Vec::with_capacity(nr_sections_total);
        let mut chord_vectors_local: Vec<SpatialVector> = Vec::with_capacity(nr_sections_total);
        let mut chord_lengths: Vec<Float> = Vec::with_capacity(nr_sections_total);
        let mut line_segment_is_virtual: Vec<bool> = Vec::with_capacity(nr_sections_total);

        for seg_index in 0..nr_segments {
            let ds = ds_locally[seg_index];
            let segment_start_distance = specified_sections_span_distance[seg_index];

            let segment_is_virtual = if let Some(data) = &self.line_segment_is_virtual {
                data[seg_index]
            } else {
                false
            };
            
            for i in 0..nr_sections_locally[seg_index] {
                let start_distance = segment_start_distance + i as Float * ds;
                let end_distance   = start_distance + ds;

                let ctrl_point_distance = 0.5 * (start_distance + end_distance);
    
                let start_point = linear_interpolation(start_distance, &specified_sections_span_distance, &self.section_points);
                let end_point   = linear_interpolation(end_distance,   &specified_sections_span_distance, &self.section_points);
    
                span_lines_local.push(SpanLine{start_point, end_point});
    
                chord_vectors_local.push(
                    linear_interpolation(ctrl_point_distance, &specified_sections_span_distance, &self.chord_vectors)
                );
    
                chord_lengths.push(chord_vectors_local.last().unwrap().length());
    
                line_segment_is_virtual.push(segment_is_virtual);
            }
        }

        let section_model = self.section_model.clone();

        SingleWing {
            span_lines_local,
            chord_vectors_local,
            chord_lengths,
            line_segment_is_virtual,
            section_model,
            non_zero_circulation_at_ends: self.non_zero_circulation_at_ends,
            input_power_model: self.input_power_model.clone(),
            internal_compartments: self.internal_compartments(),
            force_correction: self.force_correction.clone(),
            added_mass: self.added_mass.clone(),
            gyroscopic_moment: self.gyroscopic_moment(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::section_models::rotating_cylinder::RotatingCylinder;

    use super::*;

    #[test]
    fn test_wing_builder() {
        let chord_length = 5.0;
        let chord_vector = SpatialVector([chord_length, 0.0, 0.0]);
        
        let wing_builder = WingBuilder{
            section_points: vec![
                SpatialVector([0.0, 0.0, -1.0]),
                SpatialVector([0.0, 0.0, 0.0]),
                SpatialVector([0.0, 0.0, 10.0]),
                SpatialVector([0.0, 0.0, 11.0])
            ],
            chord_vectors: vec![
                chord_vector,
                chord_vector,
                chord_vector,
                chord_vector
            ],
            section_model: SectionModel::RotatingCylinder(RotatingCylinder::default()),
            line_segment_is_virtual: Some(
                vec![true, false, true]
            ),
            non_zero_circulation_at_ends: [false, false],
            nr_sections: None,
            ..Default::default()
        };

        let single_wing = wing_builder.build(12);

        dbg!(single_wing);
    }

    #[test]
    fn wing_level_force_models_from_json() {
        let wing_builder: WingBuilder = serde_json::from_str(r#"{
            "section_points": [{"x": 0.0, "y": 0.0, "z": 0.0}, {"x": 0.0, "y": 0.0, "z": 1.0}],
            "chord_vectors": [{"x": 1.0, "y": 0.0, "z": 0.0}, {"x": 1.0, "y": 0.0, "z": 0.0}],
            "section_model": {"RotatingCylinder": {}},
            "force_correction": {"lift_induced_drag_correction": 0.01},
            "added_mass": {"normal_coefficient": 1.0, "chordwise_coefficient": 1.0},
            "gyroscopic_moment": {"moment_of_inertia": 100.0}
        }"#).unwrap();

        assert_eq!(wing_builder.force_correction.lift_induced_drag_correction, 0.01);
        assert_eq!(wing_builder.added_mass.chordwise_coefficient, 1.0);
        assert_eq!(wing_builder.gyroscopic_moment.moment_of_inertia, 100.0);
    }
}