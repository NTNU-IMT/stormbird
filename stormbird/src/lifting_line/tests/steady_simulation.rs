// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests for the static simulation capabilities of the lifting line module

use crate::lifting_line::prelude::*;

use super::test_setup::RectangularWing;

#[test]
fn moment_test() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 0.5;
    let angle_of_attack = 0.0;

    let wing_builder = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        negative_span_orientation: true,
        ..Default::default()
    }.build();

    let line_force_model = wing_builder.build();

    let solver_settings = SteadySolverSettings::default();
    let wake_builder  = SteadyWakeBuilder::default();

    let velocity_input = VelocityInput{
        freestream: Vec3::new(1.2, 0.0, 0.0),
        ..Default::default()
    };

    let result = steady_solvers::solve_steady(
        &line_force_model,
        &velocity_input,
        &solver_settings,
        &wake_builder,
        vec![0.0; line_force_model.nr_span_lines()].as_slice()
    );

    let force = result.integrated_forces_sum();
    let moment = result.integrated_moments_sum();

    let cl = force.y / line_force_model.total_force_factor(velocity_input.freestream.length());

    dbg!(cl);

    // The force should be in the negative y direction as the force direction is parallel to the
    // cross product of the velocity and the span line multiplied by the strength. The strength is 
    // negative with a positive lift coeffient, by definition. The velocity vector is in the 
    // positive x direction and the span line is in the negative z direction
    assert!(force.y < 0.0, "Sign of the force in the y direction is wrong: {}", force.y);

    // The moment should be negative as a negative force in the y direction, and a span line in the
    // negative z direction will create a negative moment around the x axis
    assert!(moment.x < 0.0, "Sign of the moment in the x direction is wrong: {}", moment.x);

    let moment_arm = -moment.x / force.y;
    let moment_arm_theory = -2.5;

    let error = (moment_arm - moment_arm_theory).abs();

    assert!(error < 0.001, "Moment arm error. Moment arm = {}", moment_arm);

}
