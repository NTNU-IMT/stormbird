// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use crate::vec3::Vec3;
use crate::io_structs::prelude::*;
use crate::lifting_line::tests::test_setup::RectangularWing;

#[test]
fn compare_wake_models() {
    let line_force_model = RectangularWing{
        nr_strips: 4,
        ..Default::default()
    }.build().build();

    let velocity_input = VelocityInput{
        freestream: Vec3::new(1.2, 0.0, 0.0),
        ..Default::default()
    };

    let time_step = 0.5; 

    let mut dynamic_wake = UnsteadyWakeBuilder::default()
        .build(time_step, &line_force_model, &velocity_input);

    let steady_wake = SteadyWakeBuilder::default()
        .build(&line_force_model, &velocity_input);

    let circulation_strength_value = 1.31;

    let circulation_strength = vec![circulation_strength_value; line_force_model.nr_span_lines()];

    dynamic_wake.strengths = vec![circulation_strength_value; dynamic_wake.strengths.len()];

    let u_i_steady  = steady_wake.induced_velocities_at_control_points(&circulation_strength);
    let u_i_dynamic = dynamic_wake.induced_velocities(&line_force_model.ctrl_points(), false);

    dbg!(&steady_wake.velocity_factors);

    dbg!(&u_i_steady);
    dbg!(&u_i_dynamic);
    
}