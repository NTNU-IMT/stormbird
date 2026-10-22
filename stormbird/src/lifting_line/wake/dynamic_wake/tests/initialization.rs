// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::prelude::*;
use crate::lifting_line::wake::dynamic_wake::builder::DynamicWakeBuilder;

use stormath::type_aliases::Float;

#[test]
/// Tests that the wake built before the first time step has the length given by the
/// `initial_relative_wake_length`, for a chord length different from 1.0.
///
/// The wake is built along the chord direction. The first and last panels have fixed lengths, given
/// by the `first_panel_relative_length` and `last_panel_relative_length`, while the panels in
/// between each have the length initial_relative_wake_length * chord_length / nr_panels.
fn initial_wake_length_follows_chord_length() {
    let chord_length = 2.5;
    let nr_panels: usize = 10;

    let chord_vector = SpatialVector::from([chord_length, 0.0, 0.0]);

    let mut line_force_model_builder = LineForceModelBuilder::new(4);

    line_force_model_builder.add_wing(WingBuilder {
        section_points: vec![
            SpatialVector::from([0.0, 0.0, 0.0]),
            SpatialVector::from([0.0, 0.0, 10.0]),
        ],
        chord_vectors: vec![chord_vector, chord_vector],
        ..Default::default()
    });

    let line_force_model = line_force_model_builder.build();

    let wake_builder = DynamicWakeBuilder {
        nr_panels_per_line_element: nr_panels,
        ..Default::default()
    };

    let wake = wake_builder.build(&line_force_model);

    let middle_panels_length = wake_builder.initial_relative_wake_length * chord_length *
        (nr_panels - 2) as Float / nr_panels as Float;

    let expected_wake_length = wake_builder.first_panel_relative_length * chord_length +
        middle_panels_length +
        wake_builder.last_panel_relative_length * chord_length;

    for i_span in 0..wake.indices.nr_points_along_span {
        let first_point = wake.points[wake.indices.point_index(0, i_span)];
        let last_point = wake.points[wake.indices.point_index(nr_panels, i_span)];

        let wake_length = (last_point - first_point).length();

        assert!(
            (wake_length - expected_wake_length).abs() < 1e-6 * expected_wake_length,
            "Wake length {} is different from the expected length {}",
            wake_length,
            expected_wake_length
        );
    }
}
