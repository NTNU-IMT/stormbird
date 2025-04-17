// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Compare the result from different solvers

use crate::lifting_line::prelude::*;
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    SteadySettings,
};

use super::test_setup::RectangularWing;

use math_utils::spatial_vector::transformations::RotationType;

#[test]
/// Test that checks whether the global and body coordinate systems give the same result.
fn coordinate_systems() {
    let aspect_ratio = 5.0;
    let cl_zero_angle = 1.2;
    let angle_of_attack = 2.0_f64.to_radians();

    // Note: adding rotation in any other axis than the x-axis affects the angle of attack, and will 
    // therefore make the comparison between the "fixed" and "body" forces fail.
    let rotation = SpatialVector([
        72.0_f64.to_radians(),
        0.0,
        0.0
    ]);

    let translation = SpatialVector([100.0, 42.0, 10.0]);

    let model_builder_global = RectangularWing {
        aspect_ratio,
        cl_zero_angle,
        angle_of_attack,
        nr_strips: 32,
        ..Default::default()
    }.build();

    let mut model_builder_body = model_builder_global.clone();
    model_builder_body.output_coordinate_system = CoordinateSystem::Body;

    let steady_settings  = SteadySettings::default();

    let velocity = SpatialVector([1.2, 0.0, 0.0]);

    let time_step = 0.1;

    let mut sim_fixed = SimulationBuilder::new(
        model_builder_global.clone(),
        SimulationSettings::QuasiSteady(steady_settings.clone())
    ).build();

    let mut sim_global = sim_fixed.clone();

    let mut sim_body = SimulationBuilder::new(
        model_builder_body.clone(),
        SimulationSettings::QuasiSteady(steady_settings.clone())
    ).build();

    sim_global.line_force_model.rigid_body_motion.rotation = rotation;
    sim_body.line_force_model.rigid_body_motion.rotation = rotation;
    
    sim_global.line_force_model.rigid_body_motion.translation = translation;
    sim_body.line_force_model.rigid_body_motion.translation = translation;

    let velocity_points = sim_global.get_freestream_velocity_points();

    let input_velocity = vec![velocity; velocity_points.len()];

    let mut result_fixed = SimulationResult::default();
    let mut result_global = SimulationResult::default();
    let mut result_body = SimulationResult::default();
    for _ in 0..10 {
        result_fixed = sim_fixed.do_step(
            0.0,
            time_step,
            &input_velocity
        );

        result_global = sim_global.do_step(
            0.0,
            time_step,
            &input_velocity
        );

        result_body = sim_body.do_step(
            0.0,
            time_step,
            &input_velocity
        );
    }

    let forces_fixed = result_fixed.integrated_forces_sum();
    let forces_global = result_global.integrated_forces_sum();
    let forces_body = result_body.integrated_forces_sum();

    let moments_fixed = result_fixed.integrated_moments_sum();
    let moments_global = result_global.integrated_moments_sum();
    let moments_body = result_body.integrated_moments_sum();

    let rotation_type = RotationType::XYZ;


    let forces_global_transformed = forces_global.in_rotated_coordinate_system(
        rotation,
        rotation_type
    );

    let moment_global_transformed = moments_global.moment_in_new_coordinate_system(
        rotation,
        translation,
        forces_global,
        rotation_type
    );

    let force_error_body = (forces_body - forces_fixed).length() / forces_fixed.length();
    let moment_error_body = (moments_body - moments_fixed).length() / moments_fixed.length();

    let force_error_global = (forces_global_transformed - forces_body).length() / forces_body.length();
    let moment_error_global = (moment_global_transformed - moments_body).length() / moments_body.length();

    dbg!(forces_fixed, forces_global, forces_body, forces_global_transformed);
    dbg!(moments_fixed, moments_global, moments_body, moment_global_transformed);

    assert!(force_error_body < 1.0e-4, "force_error_body: {}", force_error_body);
    assert!(moment_error_body < 1.0e-4, "moment_error_body: {}", moment_error_body);
    assert!(force_error_global < 1.0e-4, "force_error_global: {}", force_error_global);
    assert!(moment_error_global < 1.0e-4, "moment_error_global: {}", moment_error_global);
}