// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

#[test]
/// Tests that each span point is mapped to the line element it is the start point of, except the
/// last span point of each wing, which should be mapped to the last line element of that wing.
fn test_line_element_index_from_span_point_index() {
    let line_force_model = get_example_model();

    let span_points = &line_force_model.span_points_global;
    let span_lines = &line_force_model.span_lines_global;

    let nr_wings = line_force_model.wing_indices.len();

    assert_eq!(span_points.len(), span_lines.len() + nr_wings);

    let mut span_point_index = 0;

    for wing_indices in &line_force_model.wing_indices {
        for line_index in wing_indices.clone() {
            assert_eq!(
                line_force_model.line_element_index_from_span_point_index(span_point_index),
                line_index
            );

            assert_eq!(span_points[span_point_index], span_lines[line_index].start_point);

            span_point_index += 1;
        }

        let last_line_index = wing_indices.end - 1;

        assert_eq!(
            line_force_model.line_element_index_from_span_point_index(span_point_index),
            last_line_index
        );

        assert_eq!(span_points[span_point_index], span_lines[last_line_index].end_point);

        span_point_index += 1;
    }

    assert_eq!(span_point_index, span_points.len());
}

#[test]
#[should_panic]
/// Tests that a span point index outside the model panics
fn test_line_element_index_from_span_point_index_out_of_range() {
    let line_force_model = get_example_model();

    let nr_span_points = line_force_model.span_points_global.len();

    line_force_model.line_element_index_from_span_point_index(nr_span_points);
}
