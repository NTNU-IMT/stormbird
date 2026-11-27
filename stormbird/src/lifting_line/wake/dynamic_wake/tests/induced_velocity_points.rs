// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests of which wake points get induced velocities in the stored velocity

use std::ops::Range;

use crate::lifting_line::prelude::*;
use crate::lifting_line::wake::dynamic_wake::builder::DynamicWakeBuilder;
use crate::lifting_line::wake::dynamic_wake::settings::{
    FirstWakePointsDirection,
    WakeIndices,
    points_with_induced_velocities,
};

use stormath::type_aliases::Float;

const NR_POINTS_ALONG_SPAN: usize = 5;

fn indices(nr_panels_per_line_element: usize) -> WakeIndices {
    WakeIndices {
        nr_points_along_span: NR_POINTS_ALONG_SPAN,
        nr_panels_along_span: NR_POINTS_ALONG_SPAN - 1,
        nr_panels_per_line_element,
        nr_points_per_line_element: nr_panels_per_line_element + 1,
    }
}

/// The range of flat point indices for the given range of rows
fn rows(rows: Range<usize>) -> Range<usize> {
    rows.start * NR_POINTS_ALONG_SPAN..rows.end * NR_POINTS_ALONG_SPAN
}

#[test]
fn points_with_induced_velocities_for_each_setting() {
    use FirstWakePointsDirection::*;

    let cases = [
        // No induced velocities, unless the first row follows the actual velocity
        (40, 0.0, Freestream, rows(0..0)),
        (40, 0.0, Chord, rows(0..0)),
        (40, 0.0, ActualVelocity, rows(0..2)),
        // The fraction of the streamed rows 1..=N-2
        (40, 0.25, Freestream, rows(1..11)),
        (40, 0.25, ActualVelocity, rows(0..11)),
        (40, 1.0, Freestream, rows(1..39)),
        (40, 2.0, Chord, rows(1..39)),
        (40, -1.0, Freestream, rows(0..0)),
        // A small ratio still includes the first streamed row
        (40, 0.01, Freestream, rows(1..2)),
        // With two panels, no rows are streamed
        (2, 1.0, Freestream, rows(0..0)),
        (2, 1.0, ActualVelocity, rows(0..2)),
    ];

    for (nr_panels, ratio, direction, expected) in cases {
        assert_eq!(
            points_with_induced_velocities(&indices(nr_panels), ratio, direction),
            expected,
            "{nr_panels} panels, ratio {ratio}, {direction:?}"
        );
    }
}

/// The stored velocity at the wake points after a few time steps with a constant circulation and
/// freestream velocity
fn stored_velocity(
    ratio_of_wake_affected_by_induced_velocities: Float,
    first_wake_points_direction: FirstWakePointsDirection,
) -> (Vec<SpatialVector>, SpatialVector, usize) {
    let mut line_force_model_builder = LineForceModelBuilder::new(4);

    line_force_model_builder.add_wing(WingBuilder {
        section_points: vec![
            SpatialVector::from([0.0, 0.0, 0.0]),
            SpatialVector::from([0.0, 0.0, 10.0]),
        ],
        chord_vectors: vec![SpatialVector::from([2.0, 0.0, 0.0]); 2],
        ..Default::default()
    });

    let line_force_model = line_force_model_builder.build().unwrap();

    let mut wake = DynamicWakeBuilder {
        nr_panels_per_line_element: 10,
        ratio_of_wake_affected_by_induced_velocities,
        first_wake_points_direction,
        ..Default::default()
    }.build(&line_force_model).unwrap();

    let freestream = SpatialVector::from([5.0, 0.0, 0.0]);
    let wake_points_freestream = vec![freestream; wake.points.len()];
    let ctrl_points_freestream = vec![freestream; line_force_model.nr_span_lines()];
    let circulation_strength = vec![3.0; line_force_model.nr_span_lines()];

    for _ in 0..5 {
        wake.update_before_solving(0.2, &line_force_model, &ctrl_points_freestream);
        wake.update_after_solving(&circulation_strength, &wake_points_freestream);
    }

    (wake.velocity_at_points.clone(), freestream, wake.indices.nr_points_along_span)
}

#[test]
/// The induced velocities are added to the stored velocity at exactly the points given by
/// `points_with_induced_velocities`, and nowhere else
fn induced_velocities_are_only_stored_where_they_are_used() {
    use FirstWakePointsDirection::*;

    for (ratio, direction, nr_rows_with_induced_velocities) in [
        (0.0, Freestream, 0..0),
        (0.0, ActualVelocity, 0..2),
        (0.5, Freestream, 1..5),
    ] {
        let (velocity, freestream, nr_points_along_span) = stored_velocity(ratio, direction);

        for (index, velocity) in velocity.iter().enumerate() {
            let row = index / nr_points_along_span;

            if nr_rows_with_induced_velocities.contains(&row) {
                assert_ne!(*velocity, freestream, "{direction:?}, ratio {ratio}, row {row}");
            } else {
                assert_eq!(*velocity, freestream, "{direction:?}, ratio {ratio}, row {row}");
            }
        }
    }
}
