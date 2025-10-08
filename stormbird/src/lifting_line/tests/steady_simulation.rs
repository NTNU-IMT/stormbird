// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Tests for the static simulation capabilities of the lifting line module

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    SteadySettings
};

use super::test_setup::RectangularWing;

#[test]
fn moment_test() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 0.5;
    let angle_of_attack = 0.0;

    let line_force_model_builder = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        negative_span_orientation: true,
        ..Default::default()
    }.build();

    let freestream_velocity = SpatialVector::from([1.2, 0.0, 0.0]);

    let mut sim = SimulationBuilder {
        line_force_model: line_force_model_builder,
        simulation_settings: SimulationSettings::QuasiSteady(
            SteadySettings::default()
        ) 
    }.build();

    let freestream_velocity_points = sim.get_freestream_velocity_points();

    let input_freestream_velocity = vec![freestream_velocity; freestream_velocity_points.len()];

    let result = sim.do_step(0.0, 1.0, &input_freestream_velocity);

    let force = result.integrated_forces_sum();
    let moment = result.integrated_moments_sum();

    let cl = force[1] / sim.line_force_model.total_force_factor(freestream_velocity.length());

    dbg!(cl);

    // The force should be in the negative y direction as the force direction is parallel to the
    // cross product of the velocity and the span line multiplied by the strength. The strength is 
    // negative with a positive lift coefficient, by definition. The velocity vector is in the 
    // positive x direction and the span line is in the negative z direction
    assert!(force[1] < 0.0, "Sign of the force in the y direction is wrong: {}", force[1]);

    // The moment should be negative as a negative force in the y direction, and a span line in the
    // negative z direction will create a negative moment around the x axis
    assert!(moment[0] < 0.0, "Sign of the moment in the x direction is wrong: {}", moment[0]);

    let moment_arm = -moment[0] / force[1];
    let moment_arm_theory = -2.5;

    let error = (moment_arm - moment_arm_theory).abs();

    assert!(error < 0.001, "Moment arm error. Moment arm = {}", moment_arm);

}
