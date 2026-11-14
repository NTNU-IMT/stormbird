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
use crate::section_models::dynamic_stall::{DynamicStallModel, DynamicStallModelBuilder};
use crate::error::Error;
use crate::line_force_model::span_line::SpanLine;
use crate::line_force_model::input_power::InputPowerModel;
use crate::line_force_model::wing_internal_state::InternalCompartments;
use crate::line_force_model::corrections::force::ForceCorrection;
use crate::line_force_model::added_mass::AddedMass;
use crate::line_force_model::gyroscopic_moment::GyroscopicMoment;

#[derive(Debug, Clone)]
/// Input struct to add a single wing to a line force model
pub struct SingleWing {
    /// The span lines of the wing, in the local coordinate system
    pub span_lines_local: Vec<SpanLine>,
    /// The chord vectors at the control point of each span line, in the local coordinate system
    pub chord_vectors_local: Vec<SpatialVector>,
    /// The chord length at the control point of each span line
    pub chord_lengths: Vec<Float>,
    /// Whether each span line is virtual, i.e., only part of the simulation and not of the wing
    pub line_segment_is_virtual: Vec<bool>,
    /// The section model of the wing
    pub section_model: SectionModel,
    /// Whether the circulation can be different from zero at the start and the end of the wing
    pub non_zero_circulation_at_ends: [bool; 2],
    /// Model for the input power to the wing
    pub input_power_model: InputPowerModel,
    /// The internal compartments of the wing
    pub internal_compartments: InternalCompartments,
    /// Empirical corrections of the forces on the wing
    pub force_correction: ForceCorrection,
    /// Model for the added mass forces on the wing
    pub added_mass: AddedMass,
    /// Model for the gyroscopic moment from the wing
    pub gyroscopic_moment: GyroscopicMoment,
    /// Optional dynamic stall model for the wing
    pub dynamic_stall: Option<DynamicStallModel>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A wing is specified by giving a set of points along the span where the chord vector is set.
/// Between these points the chord vector is linearly interpolated. The section model is the same
/// along the whole wing.
pub struct WingBuilder {
    /// Points along the span where the chord vectors are given
    pub section_points: Vec<SpatialVector>,
    /// The chord vector at each section point
    pub chord_vectors: Vec<SpatialVector>,
    /// The section model of the wing
    pub section_model: SectionModel,
    /// Optional specification of which span lines are virtual. None of the span lines are virtual
    /// by default.
    #[serde(default)]
    pub line_segment_is_virtual: Option<Vec<bool>>,
    /// Whether the circulation can be different from zero at the start and the end of the wing.
    /// The default is zero circulation at both ends.
    #[serde(default="WingBuilder::default_non_zero_circulation_at_ends")]
    pub non_zero_circulation_at_ends: [bool; 2],
    /// Optional number of sections for this wing, which overrides the number in the line force
    /// model builder
    #[serde(default)]
    pub nr_sections: Option<usize>,
    /// Model for the input power to the wing
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
    #[serde(default)]
    /// Optional dynamic stall model for the wing. Only used in dynamic lifting line simulations.
    /// The parameters for the stall are only allowed together with section models that build on
    /// the foil model. Not used by default.
    pub dynamic_stall: Option<DynamicStallModelBuilder>,
}

impl WingBuilder {
    /// Default value for `non_zero_circulation_at_ends`, which is zero circulation at both ends
    pub fn default_non_zero_circulation_at_ends() -> [bool; 2] {[false, false]}

    /// Returns a vector that contains the distance along the span to each section point.
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

    /// Checks that the geometry of the wing is valid, i.e., that there are at least two section
    /// points, that there is one chord vector for each section point, that the virtual line
    /// segments, if given, have one value for each segment between the section points, that the
    /// number of sections is not zero, and that the wing has a non-zero span.
    ///
    /// # Arguments
    /// * `default_nr_sections` - The number of sections to use if the wing does not specify its
    ///   own number of sections
    pub fn check_geometry(&self, default_nr_sections: usize) -> Result<(), Error> {
        let nr_section_points = self.section_points.len();

        if nr_section_points < 2 {
            return Err(Error::invalid_input(format!(
                "A wing must have at least two section points, but {} were given",
                nr_section_points
            )));
        }

        if self.chord_vectors.len() != nr_section_points {
            return Err(Error::invalid_input(format!(
                "A wing must have one chord vector for each section point. Section points: {}, \
                chord vectors: {}",
                nr_section_points, self.chord_vectors.len()
            )));
        }

        if let Some(line_segment_is_virtual) = &self.line_segment_is_virtual {
            if line_segment_is_virtual.len() != nr_section_points - 1 {
                return Err(Error::invalid_input(format!(
                    "line_segment_is_virtual must have one value for each segment between the \
                    section points, i.e., {} values, but {} were given",
                    nr_section_points - 1, line_segment_is_virtual.len()
                )));
            }
        }

        if self.nr_sections.unwrap_or(default_nr_sections) == 0 {
            return Err(Error::invalid_input(
                "The number of sections for a wing must be at least one"
            ));
        }

        let total_span_distance = self.specified_sections_span_distance()[nr_section_points - 1];

        if total_span_distance.is_nan() || total_span_distance <= 0.0 {
            return Err(Error::invalid_input(format!(
                "The span of a wing must be larger than zero, but is {}",
                total_span_distance
            )));
        }

        Ok(())
    }

    /// Returns the internal compartments of the wing.
    ///
    /// # Errors
    /// If the boundaries are not valid, or if more than one compartment is specified for a section
    /// model that does not use internal compartments.
    pub fn internal_compartments(&self) -> Result<InternalCompartments, Error> {
        let Some(boundaries) = &self.internal_compartment_boundaries else {
            return Ok(InternalCompartments::default());
        };

        let compartments = InternalCompartments::new(boundaries.clone())?;

        if compartments.nr_compartments() > 1 && !self.section_model.uses_internal_compartments() {
            return Err(Error::invalid_input(
                "Internal compartments are only used by section models with an internal flow \
                model, i.e., an active air flow foil with the PowerToInternalFlow or \
                FanSpeedToInternalFlow conversion"
            ));
        }

        Ok(compartments)
    }

    /// Returns the gyroscopic moment model of the wing.
    ///
    /// # Errors
    /// If the model is active for a section model that is not a rotating cylinder, as the internal
    /// state is then not the rotational speed of the wing.
    pub fn gyroscopic_moment(&self) -> Result<GyroscopicMoment, Error> {
        if self.gyroscopic_moment.is_active() &&
            !matches!(self.section_model, SectionModel::RotatingCylinder(_))
        {
            return Err(Error::invalid_input(
                "The gyroscopic moment model is only used together with the rotating cylinder \
                section model"
            ));
        }

        Ok(self.gyroscopic_moment.clone())
    }

    /// Checks that the dynamic stall model of the wing, if any, is valid for the section model of
    /// the wing, see [`DynamicStallModelBuilder::check`].
    pub fn check_dynamic_stall(&self) -> Result<(), Error> {
        match &self.dynamic_stall {
            Some(dynamic_stall) => dynamic_stall.check(&self.section_model),
            None => Ok(())
        }
    }

    /// Returns the builder for the dynamic stall model of the wing, if any.
    ///
    /// # Errors
    /// If the dynamic stall model is not valid for the section model, see
    /// [`check_dynamic_stall`](WingBuilder::check_dynamic_stall).
    pub fn dynamic_stall(&self) -> Result<Option<DynamicStallModelBuilder>, Error> {
        self.check_dynamic_stall()?;

        Ok(self.dynamic_stall.clone())
    }

    /// Checks all the settings of the wing, i.e., the geometry, see
    /// [`check_geometry`](WingBuilder::check_geometry), the internal compartments, the gyroscopic
    /// moment model and the dynamic stall model.
    ///
    /// # Arguments
    /// * `default_nr_sections` - The number of sections to use if the wing does not specify its
    ///   own number of sections
    pub fn check(&self, default_nr_sections: usize) -> Result<(), Error> {
        self.check_geometry(default_nr_sections)?;
        self.internal_compartments()?;
        self.gyroscopic_moment()?;
        self.check_dynamic_stall()?;

        Ok(())
    }

    /// Builds the data for a single wing
    ///
    /// # Errors
    /// If the settings of the wing are not valid, see [`check`](WingBuilder::check).
    pub fn build(&self, default_nr_sections: usize) -> Result<SingleWing, Error> {
        self.check_geometry(default_nr_sections)?;

        let nr_specified_sections = self.section_points.len();
        let nr_segments = nr_specified_sections - 1;

        let nr_sections_average = self.nr_sections.unwrap_or(default_nr_sections);

        let specified_sections_span_distance = self.specified_sections_span_distance();

        let total_span_distance = specified_sections_span_distance[nr_segments];

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

        let dynamic_stall = match self.dynamic_stall()? {
            Some(builder) => Some(builder.build(span_lines_local.len())?),
            None => None,
        };

        Ok(SingleWing {
            span_lines_local,
            chord_vectors_local,
            chord_lengths,
            line_segment_is_virtual,
            section_model,
            non_zero_circulation_at_ends: self.non_zero_circulation_at_ends,
            input_power_model: self.input_power_model.clone(),
            internal_compartments: self.internal_compartments()?,
            force_correction: self.force_correction.clone(),
            added_mass: self.added_mass.clone(),
            gyroscopic_moment: self.gyroscopic_moment()?,
            dynamic_stall,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::section_models::rotating_cylinder::RotatingCylinder;

    use super::*;

    #[test]
    fn test_wing_builder() {
        let chord_length = 5.0;
        let chord_vector = SpatialVector::from([chord_length, 0.0, 0.0]);
        
        let wing_builder = WingBuilder{
            section_points: vec![
                SpatialVector::from([0.0, 0.0, -1.0]),
                SpatialVector::from([0.0, 0.0, 0.0]),
                SpatialVector::from([0.0, 0.0, 10.0]),
                SpatialVector::from([0.0, 0.0, 11.0])
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

        let single_wing = wing_builder.build(12).unwrap();

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

    fn simple_wing_builder() -> WingBuilder {
        WingBuilder {
            section_points: vec![
                SpatialVector::from([0.0, 0.0, 0.0]),
                SpatialVector::from([0.0, 0.0, 10.0]),
            ],
            chord_vectors: vec![SpatialVector::from([1.0, 0.0, 0.0]); 2],
            ..Default::default()
        }
    }

    fn assert_invalid_input(result: Result<SingleWing, Error>) {
        assert!(matches!(result, Err(Error::InvalidInput(_))), "Expected an invalid input error");
    }

    #[test]
    fn valid_wing_builds() {
        assert!(simple_wing_builder().build(10).is_ok());
    }

    #[test]
    fn invalid_geometry_gives_errors() {
        let mut too_few_points = simple_wing_builder();
        too_few_points.section_points.truncate(1);
        too_few_points.chord_vectors.truncate(1);
        assert_invalid_input(too_few_points.build(10));

        let mut missing_chord_vector = simple_wing_builder();
        missing_chord_vector.chord_vectors.pop();
        assert_invalid_input(missing_chord_vector.build(10));

        let wrong_virtual_segments = WingBuilder {
            line_segment_is_virtual: Some(vec![false, false]),
            ..simple_wing_builder()
        };
        assert_invalid_input(wrong_virtual_segments.build(10));

        assert_invalid_input(simple_wing_builder().build(0));

        let mut zero_span = simple_wing_builder();
        zero_span.section_points[1] = zero_span.section_points[0];
        assert_invalid_input(zero_span.build(10));
    }
}
