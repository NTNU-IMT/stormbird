// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Speed test for four wing sails together

use crate::lifting_line::prelude::*;

use crate::lifting_line::solvers::{
    QuasiSteadySolverBuilder,
    linearized::Linearized
};
use crate::lifting_line::simulation_builder::{
    SimulationBuilder,
    SimulationSettings,
    QuasiSteadySettings,
};

use std::time::Instant;

#[test]
/// A test where the only goal is to evaluate the computational speed of the lifting line model.
/// 
/// This is useful when trying to implement more performance optimizations in the code.
fn wing_sails_speed_test() {
    let nr_strips = 40;

    let x_positions = [-5.0, 0.0, 5.0];
    let y_positions = [5.0, -5.0, 5.0,];
    let z_positions = [10.0, 10.0, 10.0];

    let chord_vector = SpatialVector::from([1.0, 0.0, 0.0]);

    let span = 10.0;

    let nr_tests = 5;

    let section_model = Foil{
        cl_zero_angle: 0.5,
        ..Default::default()
    };

    let mut line_force_model_builder = LineForceModelBuilder::new(nr_strips);

    let nr_wings = 3;

    for i in 0..nr_wings {
        let wing_builder = WingBuilder {
            section_points: vec![
                SpatialVector::from([x_positions[i], y_positions[i], z_positions[i]]),
                SpatialVector::from([x_positions[i], y_positions[i], z_positions[i] + span]),
            ],
            chord_vectors: vec![
                chord_vector;
                2
            ],
            section_model: SectionModel::Foil(section_model.clone()),
            non_zero_circulation_at_ends: [false, false],
            ..Default::default()
        };

        line_force_model_builder.add_wing(wing_builder);
    }

    let settings = QuasiSteadySettings{
        solver: QuasiSteadySolverBuilder::Linearized(Linearized::default()),
        ..Default::default()
    };

    let simulation_settings = SimulationSettings::QuasiSteady(
        settings
    );

    let mut sim = SimulationBuilder {
        line_force_model: line_force_model_builder.clone(),
        simulation_settings
    }.build();

    let freestream_velocity = SpatialVector::from([1.2, 0.0, 0.0]);

    let freestream_velocity_points = sim.get_freestream_velocity_points();

    let input_freestream_velocity = vec![
        freestream_velocity; freestream_velocity_points.len()
    ];

    for test_iteration in 0..nr_tests {
        let start_time = Instant::now();

        sim.previous_circulation_strength = vec![0.0; sim.line_force_model.nr_span_lines()];

        let result = sim.do_step(0.0, 1.0, &input_freestream_velocity);

        let force = result.integrated_forces_sum();
        let moment = result.integrated_moments_sum();

        let elapsed = start_time.elapsed();

        dbg!(force);
        dbg!(moment);

        println!("--- Test iteration {} ---", test_iteration);
        println!("Number of wings: {}", nr_wings);
        println!("Number of strips per wing: {}", nr_strips);
        println!("Number of iterations: {}", result.iterations);
        println!("Elapsed: {:.6?}", elapsed.as_secs_f64());
        println!();
    }

}
