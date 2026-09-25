// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use std::ops::Range;

use serde::{Deserialize, Serialize};

use stormath::statistics;
use stormath::type_aliases::Float;

use crate::{
    common_utils::results::simulation::SimulationResult,
    error::Error,
    line_force_model::LineForceModel,
    wind::environment::WindEnvironment,
};

/// Returns the index of the location that lies closest to the target. Ties are given to the lowest
/// index, so that the result never moves backwards when the target moves forwards along the span.
fn closest_index(non_dim_locations: &[Float], target: Float) -> usize {
    let mut closest = 0;
    let mut smallest_distance = Float::INFINITY;

    for (index, location) in non_dim_locations.iter().enumerate() {
        let distance = (location - target).abs();

        if distance < smallest_distance {
            smallest_distance = distance;
            closest = index;
        }
    }

    closest
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Builder for the SpanwiseMeasurementStructure, that computes the start and end index from
/// non-dimensional values of the span distance
pub struct SpanwiseMeasurementBuilder {
    #[serde(default = "SpanwiseMeasurementBuilder::default_non_dim_start_location")]
    pub non_dim_start_location: Float,
    #[serde(default = "SpanwiseMeasurementBuilder::default_non_dim_end_location")]
    pub non_dim_end_location: Float,
}

impl Default for SpanwiseMeasurementBuilder {
    fn default() -> Self {
        Self {
            non_dim_start_location: Self::default_non_dim_start_location(),
            non_dim_end_location: Self::default_non_dim_end_location(),
        }
    }
}

impl SpanwiseMeasurementBuilder {
    fn default_non_dim_start_location() -> Float {-0.25}
    fn default_non_dim_end_location() -> Float {0.25}

    /// Finds the control points of each wing that the measurement covers.
    ///
    /// The non-dimensional spanwise locations of the control points run from -0.5 at one end of a
    /// wing to 0.5 at the other, and are already stored in the line force model. Both the start and
    /// the end index are the control point that lies closest to the requested location, so a small
    /// deviation from it is expected: the control points are what the simulation actually resolves.
    ///
    /// The end index is included in the measurement, so at least one control point is always
    /// covered. When the start and the end location are the same, the measurement is the single
    /// control point that lies closest to that location.
    ///
    /// Each wing gets its own pair of indices, since the wings can be built with a different number
    /// of sections, and therefore have their control points in different places.
    pub fn build(&self, line_force_model: &LineForceModel) -> Result<SpanwiseMeasurement, Error> {
        if self.non_dim_end_location < self.non_dim_start_location {
            return Err(Error::CustomStringError(format!(
                "The end of a spanwise measurement cannot be before its start. The start location \
                 is {} and the end location is {}. Both are non-dimensional, and run from -0.5 to \
                 0.5 along the span",
                self.non_dim_start_location, self.non_dim_end_location
            )));
        }

        let non_dim_locations = &line_force_model.ctrl_point_spanwise_distance_non_dimensional;

        let nr_wings = line_force_model.wing_indices.len();

        let mut start_indices = Vec::with_capacity(nr_wings);
        let mut end_indices = Vec::with_capacity(nr_wings);

        for (wing_index, indices) in line_force_model.wing_indices.iter().enumerate() {
            let wing_locations = &non_dim_locations[indices.clone()];

            if wing_locations.is_empty() {
                return Err(Error::CustomStringError(format!(
                    "Wing {} has no control points, so there is nothing to measure on it",
                    wing_index
                )));
            }

            let start_index = closest_index(wing_locations, self.non_dim_start_location);

            // The locations increase along the span, so the index closest to the end location can
            // never come before the one closest to the start location. The max only makes sure that
            // the range stays valid, whatever rounding does at the ends
            let end_index = closest_index(wing_locations, self.non_dim_end_location).max(start_index);

            start_indices.push(start_index);
            end_indices.push(end_index);
        }

        Ok(SpanwiseMeasurement { start_indices, end_indices })
    }
}

#[derive(Debug, Default, Clone)]
/// Settings used when measuring flow characteristics from a simulation, when the intention is to
/// generate input to a controller.
///
/// The indices are local to each wing, and there is one entry per wing in the line force model that
/// the measurement was built for.
pub struct SpanwiseMeasurement {
    /// The first index in the local wing range which is included in the mean, for each wing
    pub start_indices: Vec<usize>,
    /// The last index in the local wing range which is included in the mean, for each wing. The
    /// index itself is a part of the measurement, so it can be equal to the start index
    pub end_indices: Vec<usize>,
}

impl SpanwiseMeasurement {
    pub fn measure_float_values(
        &self,
        values: &[Float],
        wing_indices: Vec<Range<usize>>,
    ) -> Vec<Float> {
    
        let nr_wings = wing_indices.len();

        assert_eq!(
            nr_wings,
            self.start_indices.len(),
            "The spanwise measurement was built for a different number of wings than the one it is \
             used on"
        );

        let mut out = vec![0.0; nr_wings];

        for i in 0..nr_wings {
            let wing_values = &values[wing_indices[i].clone()];

            // The end index is included in the measurement
            out[i] = statistics::mean(&wing_values[self.start_indices[i]..=self.end_indices[i]]);
        }

        out
    }
    
    pub fn measure_angles_of_attack(&self, simulation_result: &SimulationResult) -> Vec<Float> {
        self.measure_float_values(
            &simulation_result.force_input.angles_of_attack,
            simulation_result.wing_indices.clone()
        )
    }
    
    pub fn measure_wind_velocity_magnitude(&self, simulation_result: &SimulationResult) -> Vec<Float> {
        let velocity_magnitude: Vec<Float> = simulation_result.force_input.velocity
            .iter()
            .map(|v| v.length())
            .collect();
    
        self.measure_float_values(
            &velocity_magnitude,
            simulation_result.wing_indices.clone()
        )
    }
    
    pub fn measure_apparent_wind_direction(
        &self,
        simulation_result: &SimulationResult,
        wind_environment: &WindEnvironment,
        line_force_model: &LineForceModel,
        use_input_velocity: bool,
    ) -> Vec<Float> {
        let relevant_velocities = if use_input_velocity {
            simulation_result.felt_input_velocity_minus_rotational_motion()
        } else {
            simulation_result.felt_velocity_minus_rotational_motion()
        };
    
        let wind_directions = wind_environment
            .apparent_wind_direction_from_velocity_and_line_force_model(
                &relevant_velocities, 
                line_force_model
            );
    
        self.measure_float_values(
            &wind_directions, 
            simulation_result.wing_indices.clone()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::line_force_model::builder::LineForceModelBuilder;
    use crate::line_force_model::builder::single_wing::WingBuilder;
    use crate::line_force_model::input_power::InputPowerModel;
    use crate::section_models::SectionModel;
    use crate::section_models::foil::Foil;

    use stormath::spatial_vector::SpatialVector;

    /// A model with one wing per entry in `nr_sections`, all of them straight and along the z-axis.
    /// A wing with n sections has its control points at the non-dimensional spanwise locations
    /// -0.5 + (i + 0.5) / n, so 5 sections give -0.4, -0.2, 0.0, 0.2 and 0.4.
    fn model_with_wings(nr_sections: &[usize]) -> LineForceModel {
        let span = 30.0;
        let chord_vector = SpatialVector::from([5.0, 0.0, 0.0]);

        let mut builder = LineForceModelBuilder::new(5);

        for (wing_number, sections) in nr_sections.iter().enumerate() {
            let x = 40.0 * wing_number as Float;

            builder.add_wing(WingBuilder {
                section_points: vec![
                    SpatialVector::from([x, 0.0, 0.0]),
                    SpatialVector::from([x, 0.0, span]),
                ],
                chord_vectors: vec![chord_vector, chord_vector],
                line_segment_is_virtual: None,
                section_model: SectionModel::Foil(Foil::default()),
                non_zero_circulation_at_ends: [false, false],
                nr_sections: Some(*sections),
                input_power_model: InputPowerModel::NoPower,
            });
        }

        builder.build()
    }

    fn build(start: Float, end: Float, model: &LineForceModel) -> SpanwiseMeasurement {
        SpanwiseMeasurementBuilder {
            non_dim_start_location: start,
            non_dim_end_location: end,
        }
        .build(model)
        .unwrap()
    }

    #[test]
    fn indices_are_the_closest_control_points() {
        let model = model_with_wings(&[5]);

        // The control points are at -0.4, -0.2, 0.0, 0.2 and 0.4
        let measurement = build(-0.25, 0.25, &model);

        assert_eq!(measurement.start_indices, vec![1]);
        assert_eq!(measurement.end_indices, vec![3]);
    }

    #[test]
    fn the_whole_span_is_covered_by_the_outer_locations() {
        let model = model_with_wings(&[5]);

        let measurement = build(-0.5, 0.5, &model);

        assert_eq!(measurement.start_indices, vec![0]);
        assert_eq!(measurement.end_indices, vec![4]);
    }

    #[test]
    fn locations_outside_the_span_are_snapped_to_the_ends() {
        let model = model_with_wings(&[5]);

        let measurement = build(-2.0, 2.0, &model);

        assert_eq!(measurement.start_indices, vec![0]);
        assert_eq!(measurement.end_indices, vec![4]);
    }

    #[test]
    fn the_same_start_and_end_gives_a_single_control_point() {
        let model = model_with_wings(&[5]);

        // Exactly on the middle control point, and then just beside it
        for location in [0.0, 0.02, -0.02] {
            let measurement = build(location, location, &model);

            assert_eq!(measurement.start_indices, vec![2]);
            assert_eq!(measurement.end_indices, vec![2]);
        }
    }

    #[test]
    fn a_narrow_range_between_two_control_points_still_measures_one_of_them() {
        let model = model_with_wings(&[5]);

        // Both locations fall between the control points at 0.0 and 0.2, and closest to 0.2
        let measurement = build(0.15, 0.16, &model);

        assert_eq!(measurement.start_indices, vec![3]);
        assert_eq!(measurement.end_indices, vec![3]);
    }

    #[test]
    fn each_wing_gets_its_own_indices() {
        // The second wing has its control points at -1/3, 0.0 and 1/3
        let model = model_with_wings(&[5, 3]);

        let measurement = build(-0.25, 0.25, &model);

        assert_eq!(measurement.start_indices, vec![1, 0]);
        assert_eq!(measurement.end_indices, vec![3, 2]);
    }

    #[test]
    fn a_tie_is_given_to_the_lowest_index() {
        // Four sections put the control points at -0.375, -0.125, 0.125 and 0.375, so both -0.25
        // and 0.25 are exactly between two of them
        let model = model_with_wings(&[4]);

        let measurement = build(-0.25, 0.25, &model);

        assert_eq!(measurement.start_indices, vec![0]);
        assert_eq!(measurement.end_indices, vec![2]);
    }

    #[test]
    fn an_end_before_the_start_is_an_error() {
        let model = model_with_wings(&[5]);

        let result = SpanwiseMeasurementBuilder {
            non_dim_start_location: 0.25,
            non_dim_end_location: -0.25,
        }
        .build(&model);

        assert!(result.is_err());
    }

    #[test]
    fn the_end_index_is_included_in_the_mean() {
        let model = model_with_wings(&[5]);

        let measurement = build(-0.25, 0.25, &model);

        // Indices 1, 2 and 3 of the wing, so the mean of 20.0, 30.0 and 40.0
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];

        let measured = measurement.measure_float_values(&values, model.wing_indices.clone());

        assert_eq!(measured, vec![30.0]);
    }

    #[test]
    fn a_single_control_point_is_measured_on_its_own() {
        let model = model_with_wings(&[5]);

        let measurement = build(0.0, 0.0, &model);

        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];

        let measured = measurement.measure_float_values(&values, model.wing_indices.clone());

        assert_eq!(measured, vec![30.0]);
    }

    #[test]
    fn the_default_builder_covers_the_middle_half_of_the_span() {
        let model = model_with_wings(&[5]);

        let measurement = SpanwiseMeasurementBuilder::default().build(&model).unwrap();

        assert_eq!(measurement.start_indices, vec![1]);
        assert_eq!(measurement.end_indices, vec![3]);
    }
}
