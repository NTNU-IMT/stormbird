// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use crate::lifting_line::wake::{
    builders::{
        WakeBuilder, SteadyWakeBuilder
    }
};

use crate::lifting_line::tests::test_setup::RectangularWing;

#[test]
fn compare_wake_models() {
    let line_force_model = RectangularWing{
        nr_strips: 4,
        ..Default::default()
    }.build().build();

    let mut dynamic_wake = WakeBuilder{
        nr_panels_per_line_element: 10,
        last_panel_relative_length: 100.0,
        ..Default::default()
    }.build(&line_force_model);

    let mut steady_wake = SteadyWakeBuilder::default()
        .build(&line_force_model);

    let circulation_strength_value = 1.31;

    dynamic_wake.strengths = vec![circulation_strength_value; dynamic_wake.strengths.len()];
    steady_wake.strengths = vec![circulation_strength_value; steady_wake.strengths.len()];

    let points_to_check = line_force_model.ctrl_points_global.clone();

    let u_i_steady  = steady_wake.induced_velocities(&points_to_check);
    let u_i_dynamic = dynamic_wake.induced_velocities(&points_to_check);

    dbg!(&u_i_steady);
    dbg!(&u_i_dynamic);

    let allowable_error = 1e-4;

    for (u_i_steady, u_i_dynamic) in u_i_steady.iter().zip(u_i_dynamic.iter()) {
        assert!((*u_i_steady - *u_i_dynamic).length() < allowable_error);
    }
}