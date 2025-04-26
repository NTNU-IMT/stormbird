// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use stormath::spatial_vector::{
    SpatialVector,
    transformations::RotationType,
    geometry_functions,
};

use super::super::panel::Panel;

#[test]
/// Tests that checks that the near and far field calculations are similar for a panel with unit 
/// strength.
fn compare_near_and_far_field() {
    // Set up a panel in a local coordinate system
    let panel_points_0 = [
        SpatialVector([0.0, 0.0, 0.0]),
        SpatialVector([1.0, 0.0, 0.0]),
        SpatialVector([1.0, 1.0, 0.0]),
        SpatialVector([0.0, 1.0, 0.0]),
    ];

    // Set a control point in the local coordinate system
    let ctrl_point_0 = SpatialVector::<3>::new(5.0, 2.0, 1.0);

    // Add some rotation to test a "arbitrary" oriented panel (the angles should not matter)
    let rotation = SpatialVector::<3>::new(
        45.0_f64.to_radians(),
        -10.0_f64.to_radians(),
        0.0,
    );

    let rotation_type = RotationType::XYZ;

    let panel_points = [
        panel_points_0[0].rotate(rotation, rotation_type),
        panel_points_0[1].rotate(rotation, rotation_type),
        panel_points_0[2].rotate(rotation, rotation_type),
        panel_points_0[3].rotate(rotation, rotation_type),
    ];

    let ctrl_point = ctrl_point_0.rotate(rotation, rotation_type);

    let panel_area = geometry_functions::area_of_quadrilateral(&panel_points);

    let panel = Panel::new(
        panel_points,
        3.0,
        0.1,
    );

    assert!(panel_area - 1.0 < 1e-10);

    // Compare velocity calculations
    let u_i_near = panel.induced_velocity_as_vortex_lines_with_unit_strength(ctrl_point);
    let u_i_far  = panel.induced_velocity_as_point_doublet_with_unit_strength(ctrl_point);

    let difference = (u_i_near - u_i_far).length();

    dbg!(u_i_near);
    dbg!(u_i_far);
    dbg!(panel_area);

    let relative_error = difference / u_i_near.length();

    dbg!(relative_error);

    assert!(relative_error < 0.02);
}