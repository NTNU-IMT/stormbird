// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use crate::vec3::Vec3;
use crate::line_force_model::span_line::SpanLine;

#[test]
/// Checks whether the induced velocity from a horseshoe vortex is the same as the induced velocity
/// from a panel with the same geometry and unit strength. This should work as the last line in the
/// panel is so far away from the control point that it can be neglected.
fn compare_horseshoe_against_panel() {
    let model = PotentialTheoryModel::default();

    let span_line = SpanLine {
        start_point: Vec3::new(0.0, 0.0, 0.0),
        end_point: Vec3::new(0.0, 0.0, 1.2),
    };

    let wake_line_vector = Vec3::new(100.0, 0.0, 0.0);

    // Set up a panel in a local coordinate system
    let panel_points = [
        span_line.start_point,
        span_line.end_point,
        span_line.end_point + wake_line_vector,
        span_line.start_point + wake_line_vector,
    ];

    let ctrl_point = Vec3::new(-0.5, 0.0, 0.0);

    let u_i_panel = model.induced_velocity_from_panel_as_vortex_lines_with_unit_strength(
        &panel_points, ctrl_point, false
    );
    let u_i_horseshoe = model.induced_velocity_from_horseshoe_with_unit_strength(
        ctrl_point, &span_line, wake_line_vector
    );

    dbg!(u_i_panel);
    dbg!(u_i_horseshoe);

    let difference = (u_i_panel - u_i_horseshoe).length();

    let relative_error = difference / u_i_panel.length();

    assert!(relative_error < 1e-4);
}