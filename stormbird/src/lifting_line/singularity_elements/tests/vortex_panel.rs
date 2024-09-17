// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use math_utils::spatial_vector::SpatialVector;
use math_utils::spatial_vector::geometry_functions;



#[test]
fn compare_near_and_far_field() {
    let model = PotentialTheoryModel::default();

    // Set up a panel in a local coordinate system
    let panel_points_0 = [
        SpatialVector::<3>::new(0.0, 0.0, 0.0),
        SpatialVector::<3>::new(1.0, 0.0, 0.0),
        SpatialVector::<3>::new(1.0, 1.0, 0.0),
        SpatialVector::<3>::new(0.0, 1.0, 0.0),
    ];

    // Set a control point in the local coordinate system
    let ctrl_point_0 = SpatialVector::<3>::new(5.0, 2.0, 1.0);

    // Add some rotation to test a "arbitrary" oriented panel (the angles should not matter)
    let rotation = SpatialVector::<3>::new(
        45.0_f64.to_radians(),
        -10.0_f64.to_radians(),
        0.0,
    );

    let panel_points = [
        panel_points_0[0].rotate(rotation),
        panel_points_0[1].rotate(rotation),
        panel_points_0[2].rotate(rotation),
        panel_points_0[3].rotate(rotation),
    ];

    let ctrl_point = ctrl_point_0.rotate(rotation);

    let panel_center = 0.25 * (panel_points[0] + panel_points[1] + panel_points[2] + panel_points[3]);
    let panel_area = geometry_functions::area_of_quadrilateral(&panel_points);
    let panel_normal = geometry_functions::normal_of_quadrilateral(&panel_points);

    assert!(panel_area - 1.0 < 1e-10);

    // Compare velcoity calculations
    let u_i_near = model.induced_velocity_from_panel_as_vortex_lines_with_unit_strength(&panel_points, ctrl_point, 0.01);
    let u_i_far  = model.induced_velocity_from_panel_as_point_doublet_with_unit_strength(panel_area, panel_normal, panel_center, ctrl_point);

    let difference = (u_i_near - u_i_far).length();

    dbg!(u_i_near);
    dbg!(u_i_far);
    dbg!(panel_area);

    let relative_error = difference / u_i_near.length();

    dbg!(relative_error);

    assert!(relative_error < 0.02);
}